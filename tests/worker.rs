use frame_studio::{
    model::GameSnapshot,
    store::Store,
    worker::{self, Command, Event},
};
use image::{Rgba, RgbaImage};
use std::{sync::Arc, time::Duration};
fn temp() -> Store {
    Store::new(std::env::temp_dir().join(format!(
            "frame-worker-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )))
    .unwrap()
}

#[test]
fn queued_rename_and_favorite_commands_merge_instead_of_overwriting_each_other() {
    let store = temp();
    let meta = store
        .save_capture(&RgbaImage::new(8, 8), GameSnapshot::default(), false)
        .unwrap();
    let (tx, rx) = worker::start(store.clone());
    let _ = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    tx.send(Command::Rename(meta.id.clone(), "Nouveau nom".into()))
        .unwrap();
    tx.send(Command::ToggleFavorite(meta.id.clone())).unwrap();
    tx.send(Command::ToggleFavorite(meta.id.clone())).unwrap();
    for _ in 0..3 {
        assert!(matches!(
            rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            Event::Updated(_)
        ));
    }
    let saved = store.load_meta(&meta.id).unwrap();
    assert_eq!(saved.title, "Nouveau nom");
    assert!(!saved.favorite);
    tx.send(Command::ToggleFavorite(meta.id.clone())).unwrap();
    tx.send(Command::Rename(meta.id.clone(), "Encore mieux".into()))
        .unwrap();
    for _ in 0..2 {
        assert!(matches!(
            rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            Event::Updated(_)
        ));
    }
    let saved = store.load_meta(&meta.id).unwrap();
    assert!(saved.favorite);
    assert_eq!(saved.title, "Encore mieux");
    drop(tx);
    drop(rx);
    std::fs::remove_dir_all(store.root).unwrap();
}
#[test]
fn corrupted_thumbnail_falls_back_to_original_without_affecting_editor_load() {
    let store = temp();
    let image = RgbaImage::from_pixel(16, 10, Rgba([20, 80, 140, 255]));
    let good = store
        .save_capture(&image, GameSnapshot::default(), false)
        .unwrap();
    std::fs::write(store.thumb_path(&good.id).unwrap(), b"corrupt").unwrap();
    let (tx, rx) = worker::start(store.clone());
    let _ = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    tx.send(Command::Thumbnail(good.id.clone())).unwrap();
    tx.send(Command::Load(good.id.clone())).unwrap();
    match rx.recv_timeout(Duration::from_secs(2)).unwrap() {
        Event::Thumbnail(id, img) => {
            assert_eq!(id, good.id);
            assert_eq!(img.get_pixel(0, 0).0, [20, 80, 140, 255]);
        }
        _ => panic!("corrupt thumbnail was not recovered from original"),
    }
    match rx.recv_timeout(Duration::from_secs(2)).unwrap() {
        Event::Loaded(id, _, _) => assert_eq!(id, good.id),
        _ => panic!("valid editor load was disrupted"),
    }
    drop(tx);
    drop(rx);
    std::fs::remove_dir_all(&store.root).unwrap();
}
#[test]
fn worker_capture_really_saves_and_emits_a_success_preview() {
    let store = temp();
    let (tx, rx) = worker::start(store.clone());
    let _ = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    tx.send(Command::Capture(
        Arc::new(RgbaImage::from_pixel(16, 10, Rgba([10, 40, 90, 255]))),
        GameSnapshot::default(),
        false,
    ))
    .unwrap();
    match rx.recv_timeout(Duration::from_secs(2)).unwrap() {
        Event::Saved(meta, thumb) => {
            assert!(store.image_path(&meta.id).unwrap().is_file());
            assert_eq!(thumb.get_pixel(0, 0).0, [10, 40, 90, 255]);
        }
        _ => panic!("capture was not saved"),
    }
    drop(tx);
    drop(rx);
    std::fs::remove_dir_all(&store.root).unwrap();
}
#[test]
fn failure_identifies_its_operation_and_capture_id() {
    let store = temp();
    let (tx, rx) = worker::start(store.clone());
    let _ = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    tx.send(Command::Thumbnail("1-404".into())).unwrap();
    match rx.recv_timeout(Duration::from_secs(2)).unwrap() {
        Event::Failed(kind, _) => assert_eq!(kind, worker::Failure::Thumbnail("1-404".into())),
        _ => panic!("missing correlated thumbnail failure"),
    }
    tx.send(Command::Load("2-404".into())).unwrap();
    match rx.recv_timeout(Duration::from_secs(2)).unwrap() {
        Event::Failed(kind, _) => assert_eq!(kind, worker::Failure::Load("2-404".into())),
        _ => panic!("missing correlated load failure"),
    }
    drop(tx);
    drop(rx);
    std::fs::remove_dir_all(&store.root).unwrap();
}
