#![forbid(deprecated)]

use macros::From;
use macros::Into;

#[test]
fn derives_each_conversion_independently() {
    #[derive(From)]
    struct RequestBody(Vec<u8>);

    #[derive(Into)]
    struct ResponseBody(Vec<u8>);

    let request = RequestBody::from(vec![1, 2, 3]);
    assert_eq!(request.0, [1, 2, 3]);
    assert_eq!(Vec::from(ResponseBody(vec![4, 5])), [4, 5]);
}

#[test]
fn derives_both_conversions_for_one_type() {
    #[derive(From, Into)]
    struct Payload(Vec<u8>);

    let payload = Payload::from(vec![1, 2, 3]);
    let bytes: Vec<_> = payload.into();
    assert_eq!(bytes, [1, 2, 3]);
}

#[test]
fn resolves_self_in_field_types_and_array_lengths() {
    #[derive(From, Into)]
    struct Node(Vec<Self>);

    #[derive(From, Into)]
    struct Packet([u8; Self::WIDTH]);

    impl Packet {
        const WIDTH: usize = 2;
    }

    let root = Node::from(vec![Node::from(Vec::new())]);
    assert_eq!(Vec::from(root).len(), 1);
    assert_eq!(<[u8; 2]>::from(Packet::from([3, 4])), [3, 4]);
}

#[test]
fn resolves_self_supplied_by_a_field_type_macro() {
    macro_rules! children {
        () => { Vec<Self> };
    }

    #[derive(Into)]
    struct Tree(children!());

    assert!(Vec::from(Tree(Vec::new())).is_empty());
}

#[test]
fn keeps_the_wrapper_in_scope_when_a_const_block_reuses_its_name() {
    #[derive(Into)]
    struct Packet(
        [u8; {
            struct Packet;
            impl Packet {
                const WIDTH: usize = 1;
            }
            Self::WIDTH + Packet::WIDTH
        }],
    );

    impl Packet {
        const WIDTH: usize = 2;
    }

    assert_eq!(<[u8; 3]>::from(Packet([1, 2, 3])), [1, 2, 3]);
}
