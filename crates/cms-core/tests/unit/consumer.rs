use super::Channel;

#[test]
fn exposes_only_the_current_channels() {
    assert_eq!(Channel::try_from("memos").unwrap(), Channel::Memos);
    assert_eq!(Channel::try_from("moment").unwrap(), Channel::Moment);
    assert_eq!(Channel::try_from("knowledge").unwrap(), Channel::Knowledge);
    assert!(Channel::try_from("archive").is_err());
}
