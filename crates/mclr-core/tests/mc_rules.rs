use mclr_core::{loader, mcver};
use serde::Deserialize;
use std::cmp::Ordering;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    compare: Vec<(String, String, i32)>,
    is_release: Vec<(String, bool)>,
    drop_of: Vec<(String, u32)>,
    major_key: Vec<(String, Option<String>)>,
    parse_loader: Vec<(String, Option<String>)>,
    expand: Vec<(Vec<String>, String, Vec<String>)>,
    supported: Vec<(String, Vec<String>)>,
    incompatible: Vec<(String, Vec<(String, String)>)>,
}

#[test]
fn test_mc_rules_golden_fixture() {
    let json_str = include_str!("fixtures/mc_rules.json");
    let fixture: Fixture =
        serde_json::from_str(json_str).expect("failed to deserialize mc_rules.json fixture");

    for (a, b, expected) in fixture.compare {
        let ord = mcver::compare(&a, &b);
        let expected_ord = match expected {
            -1 => Ordering::Less,
            0 => Ordering::Equal,
            1 => Ordering::Greater,
            other => panic!("unexpected expected ordering {other}"),
        };
        assert_eq!(
            ord, expected_ord,
            "mcver::compare({a:?}, {b:?}) was {ord:?}, expected {expected_ord:?}"
        );
    }

    for (id, expected) in fixture.is_release {
        let actual = mcver::is_release(&id);
        assert_eq!(
            actual, expected,
            "mcver::is_release({id:?}) was {actual}, expected {expected}"
        );
    }

    for (id, expected) in fixture.drop_of {
        let actual = mcver::drop_of(&id);
        assert_eq!(
            actual, expected,
            "mcver::drop_of({id:?}) was {actual}, expected {expected}"
        );
    }

    for (id, expected) in fixture.major_key {
        let actual = mcver::major_key(&id).map(str::to_string);
        assert_eq!(
            actual, expected,
            "mcver::major_key({id:?}) was {actual:?}, expected {expected:?}"
        );
    }

    for (label, expected) in fixture.parse_loader {
        let actual = loader::Loader::parse(&label).map(|l| l.id().to_string());
        assert_eq!(
            actual, expected,
            "loader::Loader::parse({label:?}) was {actual:?}, expected {expected:?}"
        );
    }

    for (labels, mc, expected) in fixture.expand {
        let actual = loader::expand_mod_loaders(&labels, &mc);
        assert_eq!(
            actual, expected,
            "loader::expand_mod_loaders({labels:?}, {mc:?}) was {actual:?}, expected {expected:?}"
        );
    }

    for (mc, expected) in fixture.supported {
        let actual: Vec<String> = loader::supported_for(&mc)
            .into_iter()
            .map(|l| l.id().to_string())
            .collect();
        assert_eq!(
            actual, expected,
            "loader::supported_for({mc:?}) was {actual:?}, expected {expected:?}"
        );
    }

    for (mc, expected) in fixture.incompatible {
        let actual: Vec<(String, String)> = loader::incompatible_pairs(&mc)
            .into_iter()
            .map(|(a, b)| (a.id().to_string(), b.id().to_string()))
            .collect();
        assert_eq!(
            actual, expected,
            "loader::incompatible_pairs({mc:?}) was {actual:?}, expected {expected:?}"
        );
    }
}
