use super::*;
use shikifsrs_elden_ring::{
    GroupObservation,
    character_sets::CharacterIdentity,
    menu_state::{FrontendStateObservation, LoadingPulseObservation, MenuTransitionObservation},
    objects::{ObjectKind, ObjectObservation},
    task_group_state::PhaseGroupStateObservation,
};
use shikifsrs_runtime::Error;

#[test]
fn original_two_second_sample_threshold_and_missing_player_reset_are_preserved() {
    for interval in [0, 10, 50, 100, 500, 2000, 3000] {
        let mut delay = WorldPlayerStopDelay::new(interval);
        let required = (2000 / interval.max(1)).max(1);
        assert_eq!(delay.required, required);
        for _ in 1..required {
            assert!(!delay.observe(true));
        }
        assert!(delay.observe(true));
        assert!(!delay.observe(false));
        assert_eq!(delay.consecutive, 0);
        for _ in 1..required {
            assert!(!delay.observe(true));
        }
        assert!(delay.observe(true));
    }
}

#[test]
fn diagnostic_failure_cannot_reset_the_primary_stop_counter() {
    let mut delay = WorldPlayerStopDelay::new(500);
    for index in 0..4 {
        let snapshot = TitleGateSnapshot::from_observations(
            Ok(true),
            Err(Error::InvalidProfile("fixture changing menu diagnostics")),
        );
        assert!(snapshot.world_player && snapshot.primary_observation_ok);
        assert!(!snapshot.diagnostics_available);
        assert_eq!(snapshot.groups, [None; 4]);
        assert_eq!(delay.observe(snapshot.world_player), index == 3);
    }
}

fn raw() -> TitleRuntimeObservation {
    let object = |address, kind| ObjectObservation { address, kind };
    TitleRuntimeObservation {
        module_base: 0x140000000,
        epoch: 1,
        frontend: Some(FrontendStateObservation {
            module_base: 0x140000000,
            epoch: 1,
            frontend: object(0x1000, ObjectKind::Frontend),
            native_state: -123,
        }),
        menu: Some(MenuTransitionObservation {
            menu: object(0x2000, ObjectKind::MenuManager),
            native_selector: -1,
            value_bits: [0; 4],
        }),
        loading_pulses: Some(LoadingPulseObservation {
            helper: object(0x3000, ObjectKind::LoadingHelper),
            current: 255,
            previous: 17,
        }),
        // The independent primary result, rather than this optional diagnostic copy, drives stop.
        primary_player: None,
        groups: [Some(PhaseGroupStateObservation {
            group: GroupObservation {
                registration_index: 0,
                native_pointer: 0x4000,
                native_words: [0; 4],
            },
            manager_address: 0x5000,
            scheduler_address: 0x6000,
            records_start: 0x7000,
            records_end: 0x7088,
            record_address: 0x7000,
            native_group_id: 0xdeadbeef,
            native_enabled: 255,
        }); 4],
    }
}

#[test]
fn raw_signed_states_and_unknown_bytes_remain_diagnostics_only() {
    let snapshot = TitleGateSnapshot::from_observations(Ok(true), Ok(raw()));
    assert_eq!(snapshot.groups, [Some(255); 4]);
    assert_eq!(snapshot.frontend_state, Some(-123));
    assert_eq!(snapshot.menu_selector, Some(-1));
    assert_eq!(snapshot.loading_pulses, Some([255, 17]));
    assert!(snapshot.world_player && snapshot.diagnostics_available);
    assert!(snapshot.summary().contains("frontend_raw=Some(-123)"));
    let missing = TitleGateSnapshot::from_observations(Ok(false), Ok(raw()));
    assert!(!missing.world_player);
    let mut fake_primary = raw();
    fake_primary.primary_player = Some(CharacterIdentity {
        address: 1,
        world_address: 2,
        native_identifier: 3,
        epoch: 1,
    });
    let rejected = TitleGateSnapshot::from_observations(
        Err(Error::InvalidProfile("fixture primary unavailable")),
        Ok(fake_primary),
    );
    assert!(!rejected.world_player && !rejected.primary_observation_ok);
    assert!(rejected.diagnostics_available);
}

#[test]
fn primary_identity_changes_without_absence_do_not_restart_original_stop_delay() {
    let mut delay = WorldPlayerStopDelay::new(1000);
    // Native identity is deliberately not a stop-policy key: the owned original uses presence.
    assert!(!delay.observe(true));
    let mut diagnostics = raw();
    diagnostics.primary_player = Some(CharacterIdentity {
        address: 9,
        world_address: 2,
        native_identifier: 100,
        epoch: 2,
    });
    let snapshot = TitleGateSnapshot::from_observations(Ok(true), Ok(diagnostics));
    assert!(delay.observe(snapshot.world_player));
}

#[test]
fn test_executable_is_rejected_before_any_er_loaded_layout_selection() {
    // current_exe is this original fixture's own test runner, never a game/process target.
    assert!(fixed_er_bytes().is_err());
    assert!(detect_er_runtime_layout().is_none());
}
