use cra::*;

#[test]
fn test_tar_reader() {
    let reader = ArcReader::new(include_bytes!("test.tar"));
    assert!(reader.is_ok());
    let reader = reader.unwrap();
    assert_eq!(reader.format(), ArcFormat::Tar);
    assert_eq!(
        reader.entries(),
        &vec![
            ArcEntry::File("hmmm".into(), "twoja stara\n".into()),
            ArcEntry::Directory("uwu/".into()),
            ArcEntry::File("uwu/owo".into(), vec![]),
        ]
    );
}

#[test]
fn test_tar_writer() {
    // TODO write an actual test instead of just testing whether it runs at all
    let mut writer = ArcWriter::new(ArcFormat::Tar);
    writer.push(ArcEntry::File("hmmm".into(), "twoja stara\n".into()));
    writer.push(ArcEntry::Directory("uwu/".into()));
    writer.push(ArcEntry::File("uwu/owo".into(), vec![]));
    writer.archive().unwrap();
}
