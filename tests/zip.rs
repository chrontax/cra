use cra::*;

#[test]
fn test_zip_reader() {
    let reader = ArcReader::new(include_bytes!("test.zip"));
    assert!(reader.is_ok());
    let reader = reader.unwrap();
    assert_eq!(reader.format(), ArcFormat::Zip);
    assert_eq!(
        reader.entries(),
        &vec![
            ArcEntry::Directory("uwu/".into()),
            ArcEntry::File("uwu/owo".into(), vec![]),
            ArcEntry::File("hmmm".into(), "twoja stara\n".into())
        ]
    );
}

#[test]
fn test_zip_writer() {
    // TODO write an actual test instead of just testing whether it runs at all
    let mut writer = ArcWriter::new(ArcFormat::Zip);
    writer.push(ArcEntry::Directory("uwu/".into()));
    writer.push(ArcEntry::File("uwu/owo".into(), vec![]));
    writer.push(ArcEntry::File("hmmm".into(), "twoja stara\n".into()));
    writer.archive().unwrap();
}
