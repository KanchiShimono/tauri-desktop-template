use std::cell::Cell;

use crate::presence::Presence;

struct NonDefault;

const fn assert_const_methods_are_usable() {
    assert!(Presence::Present(1).is_present());
    assert!(Presence::<i32>::Absent.is_absent());

    match Presence::Present(1).as_ref() {
        Presence::Absent => panic!("expected a present value"),
        Presence::Present(value) => assert!(*value == 1),
    }

    let mut presence = Presence::Present(1);
    match presence.as_mut() {
        Presence::Absent => panic!("expected a present value"),
        Presence::Present(value) => {
            *value += 1;
            assert!(*value == 2);
        }
    }
}

const _: () = assert_const_methods_are_usable();

#[test]
fn default_is_absent() {
    assert_eq!(Presence::<()>::default(), Presence::Absent);
}

#[test]
fn default_does_not_require_inner_default() {
    assert!(Presence::<NonDefault>::default().is_absent());
}

#[test]
fn is_present_matches_variant() {
    for (presence, expected) in [(Presence::Absent, false), (Presence::Present(1), true)] {
        assert_eq!(presence.is_present(), expected);
    }
}

#[test]
fn is_absent_matches_variant() {
    for (presence, expected) in [(Presence::Absent, true), (Presence::Present(1), false)] {
        assert_eq!(presence.is_absent(), expected);
    }
}

#[test]
fn as_ref_preserves_presence() {
    let absent = Presence::<i32>::Absent;
    let present = Presence::Present(1);

    assert_eq!(absent.as_ref(), Presence::Absent);
    assert_eq!(present.as_ref(), Presence::Present(&1));
}

#[test]
fn as_mut_preserves_absence() {
    let mut absent = Presence::<i32>::Absent;

    assert!(absent.as_mut().is_absent());
}

#[test]
fn as_mut_allows_mutation() {
    let mut present = Presence::Present(1);

    if let Presence::Present(value) = present.as_mut() {
        *value += 1;
    }
    assert_eq!(present, Presence::Present(2));
}

#[test]
fn as_deref_preserves_presence() {
    let absent = Presence::<String>::Absent;
    let present = Presence::Present(String::from("value"));

    assert_eq!(absent.as_deref(), Presence::Absent);
    assert_eq!(present.as_deref(), Presence::Present("value"));
}

#[test]
fn as_deref_mut_preserves_absence() {
    let mut absent = Presence::<String>::Absent;

    assert!(absent.as_deref_mut().is_absent());
}

#[test]
fn as_deref_mut_allows_target_mutation() {
    let mut present = Presence::Present(String::from("value"));

    if let Presence::Present(value) = present.as_deref_mut() {
        value.make_ascii_uppercase();
    }
    assert_eq!(present, Presence::Present(String::from("VALUE")));
}

#[test]
fn map_transforms_only_a_present_value() {
    for (presence, expected, expected_calls) in [
        (Presence::Absent, Presence::Absent, 0),
        (
            Presence::Present(2),
            Presence::Present(String::from("2")),
            1,
        ),
    ] {
        let calls = Cell::new(0);
        let actual = presence.map(|value| {
            calls.set(calls.get() + 1);
            value.to_string()
        });

        assert_eq!(actual, expected);
        assert_eq!(calls.get(), expected_calls);
    }
}

#[test]
fn map_or_returns_default_or_mapped_value() {
    for (presence, expected) in [(Presence::Absent, "default"), (Presence::Present(2), "2")] {
        let actual = presence.map_or(String::from("default"), |value| value.to_string());

        assert_eq!(actual, expected);
    }
}

#[test]
fn map_or_evaluates_default_eagerly() {
    let calls = Cell::new(0);
    let actual = Presence::Present(2).map_or(
        {
            calls.set(calls.get() + 1);
            String::from("default")
        },
        |value| value.to_string(),
    );

    assert_eq!(actual, "2");
    assert_eq!(calls.get(), 1);
}

#[test]
fn map_or_else_evaluates_only_selected_branch() {
    for (presence, expected, expected_default_calls, expected_function_calls) in [
        (Presence::Absent, "default", 1, 0),
        (Presence::Present(2), "2", 0, 1),
    ] {
        let default_calls = Cell::new(0);
        let function_calls = Cell::new(0);
        let actual = presence.map_or_else(
            || {
                default_calls.set(default_calls.get() + 1);
                String::from("default")
            },
            |value| {
                function_calls.set(function_calls.get() + 1);
                value.to_string()
            },
        );

        assert_eq!(actual, expected);
        assert_eq!(default_calls.get(), expected_default_calls);
        assert_eq!(function_calls.get(), expected_function_calls);
    }
}

#[test]
fn and_then_calls_function_only_when_present() {
    for (presence, expected, expected_calls) in [
        (Presence::Absent, Presence::Absent, 0),
        (Presence::Present(0), Presence::Absent, 1),
        (
            Presence::Present(2),
            Presence::Present(String::from("2")),
            1,
        ),
    ] {
        let calls = Cell::new(0);
        let actual = presence.and_then(|value| {
            calls.set(calls.get() + 1);
            if value == 0 {
                Presence::Absent
            } else {
                Presence::Present(value.to_string())
            }
        });

        assert_eq!(actual, expected);
        assert_eq!(calls.get(), expected_calls);
    }
}

#[test]
fn or_returns_first_present_value() {
    for (presence, fallback, expected) in [
        (Presence::Absent, Presence::Absent, Presence::Absent),
        (Presence::Absent, Presence::Present(2), Presence::Present(2)),
        (Presence::Present(1), Presence::Absent, Presence::Present(1)),
        (
            Presence::Present(1),
            Presence::Present(2),
            Presence::Present(1),
        ),
    ] {
        let actual = presence.or(fallback);

        assert_eq!(actual, expected);
    }
}

#[test]
fn or_evaluates_fallback_eagerly() {
    let calls = Cell::new(0);
    let actual = Presence::Present(1).or({
        calls.set(calls.get() + 1);
        Presence::Present(2)
    });

    assert_eq!(actual, Presence::Present(1));
    assert_eq!(calls.get(), 1);
}

#[test]
fn or_else_evaluates_fallback_only_when_absent() {
    for (presence, fallback, expected, expected_calls) in [
        (
            Presence::Present(1),
            Presence::Absent,
            Presence::Present(1),
            0,
        ),
        (Presence::Absent, Presence::Absent, Presence::Absent, 1),
        (
            Presence::Absent,
            Presence::Present(2),
            Presence::Present(2),
            1,
        ),
    ] {
        let calls = Cell::new(0);
        let actual = presence.or_else(|| {
            calls.set(calls.get() + 1);
            fallback
        });

        assert_eq!(actual, expected);
        assert_eq!(calls.get(), expected_calls);
    }
}

#[test]
fn unwrap_or_returns_value_or_default() {
    for (presence, expected) in [(Presence::Present(1), 1), (Presence::Absent, 2)] {
        let actual = presence.unwrap_or(2);

        assert_eq!(actual, expected);
    }
}

#[test]
fn unwrap_or_evaluates_default_eagerly() {
    let calls = Cell::new(0);
    let actual = Presence::Present(1).unwrap_or({
        calls.set(calls.get() + 1);
        2
    });

    assert_eq!(actual, 1);
    assert_eq!(calls.get(), 1);
}

#[test]
fn unwrap_or_else_evaluates_default_only_when_absent() {
    for (presence, expected, expected_calls) in
        [(Presence::Present(1), 1, 0), (Presence::Absent, 2, 1)]
    {
        let calls = Cell::new(0);
        let actual = presence.unwrap_or_else(|| {
            calls.set(calls.get() + 1);
            2
        });

        assert_eq!(actual, expected);
        assert_eq!(calls.get(), expected_calls);
    }
}

#[test]
fn unwrap_or_default_returns_value_or_default() {
    for (presence, expected) in [
        (Presence::Present(String::from("value")), "value"),
        (Presence::Absent, ""),
    ] {
        assert_eq!(presence.unwrap_or_default(), expected);
    }
}

#[test]
fn ok_or_returns_value_or_error() {
    for (presence, expected) in [
        (Presence::Present(1), Result::Ok(1)),
        (Presence::Absent, Result::Err("absent")),
    ] {
        let actual = presence.ok_or("absent");

        assert_eq!(actual, expected);
    }
}

#[test]
fn ok_or_evaluates_error_eagerly() {
    let calls = Cell::new(0);
    let actual = Presence::Present(1).ok_or({
        calls.set(calls.get() + 1);
        "absent"
    });

    assert_eq!(actual, Result::Ok(1));
    assert_eq!(calls.get(), 1);
}

#[test]
fn ok_or_else_evaluates_error_only_when_absent() {
    for (presence, expected, expected_calls) in [
        (Presence::Present(1), Result::Ok(1), 0),
        (Presence::Absent, Result::Err("absent"), 1),
    ] {
        let calls = Cell::new(0);
        let actual = presence.ok_or_else(|| {
            calls.set(calls.get() + 1);
            "absent"
        });

        assert_eq!(actual, expected);
        assert_eq!(calls.get(), expected_calls);
    }
}

#[test]
fn transpose_preserves_absence_success_and_failure() {
    for (presence, expected) in [
        (Presence::Absent, Result::Ok(Presence::Absent)),
        (
            Presence::Present(Result::Ok(1)),
            Result::Ok(Presence::Present(1)),
        ),
        (
            Presence::Present(Result::Err("failure")),
            Result::Err("failure"),
        ),
    ] {
        assert_eq!(presence.transpose(), expected);
    }
}

#[test]
fn option_to_presence_preserves_state() {
    for (option, presence) in [
        (Option::None, Presence::Absent),
        (Option::Some(1), Presence::Present(1)),
    ] {
        assert_eq!(Presence::from(option), presence);
    }
}

#[test]
fn presence_to_option_preserves_state() {
    for (presence, option) in [
        (Presence::Absent, Option::None),
        (Presence::Present(1), Option::Some(1)),
    ] {
        assert_eq!(Option::from(presence), option);
    }
}

#[test]
fn nested_option_conversion_round_trips() {
    for presence in [
        Presence::Absent,
        Presence::Present(Option::None),
        Presence::Present(Option::Some(1)),
    ] {
        let option = Option::from(presence.clone());

        assert_eq!(Presence::from(option), presence);
    }
}
