    fn local_only(mut estate: Estate) -> Estate {
        estate
            .model_bindings
            .retain(|binding| binding.class != estate_schema::ModelClass::Frontier);
        estate
    }

    #[test]
    fn backup_and_restore_refuse_catalog_disagreement() {
        let estate = example();
        let root = tmp();
        let state = root.join("state");
        apply_with_profile_dir(&estate, &state, &root).unwrap();
        std::fs::write(state.join("catalog.json"), "{\"frontier\":{\"model\":\"\"}}\n").unwrap();
        let (archive, _) =
            backup_cell(&state, None, &root.join("backups"), Some(&estate)).unwrap();
        std::fs::write(
            state.join("catalog.json"),
            "{\"frontier\":{\"model\":\"not-the-card\"}}\n",
        )
        .unwrap();
        let before = list_cell_backups(&root.join("backups")).unwrap().len();
        let err = backup_cell(&state, None, &root.join("backups"), Some(&estate)).unwrap_err();
        assert!(err.to_string().contains("refuse:frontier-model"), "{err}");
        assert!(err.to_string().contains("model=not-the-card"), "{err}");
        assert!(err.to_string().contains("binding model=-"), "{err}");
        assert_eq!(list_cell_backups(&root.join("backups")).unwrap().len(), before);
        let dest = root.join("restored");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("placement-actual.json"), "sentinel\n").unwrap();
        std::fs::write(
            archive.join("cell").join("catalog.json"),
            "{\"frontier\":{\"model\":\"not-the-card\"}}\n",
        )
        .unwrap();
        let dry = restore_cell(&archive, &dest, None, Some(&estate), true).unwrap();
        assert!(!dry.writes);
        assert!(dry.would_refuse);
        assert!(
            dry.refuses.iter().any(|line| {
                line.contains("refuse:frontier-model") && line.contains("not-the-card")
            }),
            "{:?}",
            dry.refuses
        );
        let live = restore_cell(&archive, &dest, None, Some(&estate), false).unwrap();
        assert!(!live.writes);
        assert!(live.would_refuse);
        assert_eq!(
            std::fs::read_to_string(dest.join("placement-actual.json")).unwrap(),
            "sentinel\n"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn backup_and_restore_refuse_frontier_invent_without_a_binding() {
        let estate = example();
        let root = tmp();
        let state = root.join("state");
        apply_with_profile_dir(&estate, &state, &root).unwrap();
        let feed = state.join("feed").join("accepted");
        std::fs::create_dir_all(&feed).unwrap();
        std::fs::write(
            feed.join("pack.json"),
            "{\"source_drivers\":[\"frontier\",\"local\"]}\n",
        )
        .unwrap();
        let (archive, _) = backup_cell(&state, None, &root.join("kept"), Some(&estate)).unwrap();
        assert!(archive
            .join("cell")
            .join("feed")
            .join("accepted")
            .join("pack.json")
            .is_file());
        let local = local_only(estate);
        let err = backup_cell(&state, None, &root.join("refused"), Some(&local)).unwrap_err();
        assert!(err.to_string().contains("refuse:frontier-invent"), "{err}");
        assert!(err.to_string().contains("source_driver"), "{err}");
        assert!(!err.to_string().contains("not-the-card"), "{err}");
        assert!(!root.join("refused").exists());
        std::fs::remove_dir_all(state.join("feed")).unwrap();
        std::fs::write(
            state.join("catalog.json"),
            "{\"frontier\":{\"model\":\"not-the-card\"}}\n",
        )
        .unwrap();
        let err =
            backup_cell(&state, None, &root.join("refused-catalog"), Some(&local)).unwrap_err();
        assert!(err.to_string().contains("refuse:frontier-invent"), "{err}");
        assert!(err.to_string().contains("catalog model"), "{err}");
        assert!(!err.to_string().contains("not-the-card"), "{err}");
        assert!(!root.join("refused-catalog").exists());
        std::fs::remove_file(state.join("catalog.json")).unwrap();
        std::fs::create_dir_all(&feed).unwrap();
        std::fs::write(feed.join("pack.json"), "{\"source_drivers\":[\"local\"]}\n").unwrap();
        let (_local_archive, _) =
            backup_cell(&state, None, &root.join("local-ok"), Some(&local)).unwrap();
        let dest = root.join("restored");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("placement-actual.json"), "sentinel\n").unwrap();
        let report = restore_cell(&archive, &dest, None, Some(&local), false).unwrap();
        assert!(!report.writes);
        assert!(report.would_refuse);
        assert!(
            report.refuses.iter().any(|line| {
                line.contains("refuse:frontier-invent") && line.contains("source_driver")
            }),
            "{:?}",
            report.refuses
        );
        assert!(
            report.refuses.iter().all(|line| !line.contains("not-the-card")),
            "{:?}",
            report.refuses
        );
        assert_eq!(
            std::fs::read_to_string(dest.join("placement-actual.json")).unwrap(),
            "sentinel\n"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn backup_and_restore_refuse_snapshot_sacred_mismatch() {
        let estate = example();
        let root = tmp();
        let state = root.join("state");
        apply_with_profile_dir(&estate, &state, &root).unwrap();
        let mut snap = load_desired_snapshot(&state).unwrap().unwrap();
        snap.sacred_exclusions
            .push(estate_schema::SacredExclusion {
                id: "extra-exclusion".into(),
                aliases: Vec::new(),
                reason: "not on the estate".into(),
            });
        write_desired_snapshot(&state, &snap).unwrap();
        let err = backup_cell(&state, None, &root.join("backups"), Some(&estate)).unwrap_err();
        assert!(err.to_string().contains("refuse:sacred-mismatch"), "{err}");
        assert!(err.to_string().contains("desired snapshot"), "{err}");
        assert!(!root.join("backups").exists());
        snap.sacred_exclusions.pop();
        write_desired_snapshot(&state, &snap).unwrap();
        let (archive, _) =
            backup_cell(&state, None, &root.join("backups"), Some(&estate)).unwrap();
        let mut archived = load_desired_snapshot(&archive.join("cell")).unwrap().unwrap();
        archived
            .sacred_exclusions
            .push(estate_schema::SacredExclusion {
                id: "extra-exclusion".into(),
                aliases: Vec::new(),
                reason: "not on the estate".into(),
            });
        write_desired_snapshot(&archive.join("cell"), &archived).unwrap();
        let dest = root.join("restored");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("placement-actual.json"), "sentinel\n").unwrap();
        let dry = restore_cell(&archive, &dest, None, Some(&estate), true).unwrap();
        assert!(!dry.writes);
        assert!(dry.would_refuse);
        assert!(
            dry.refuses
                .iter()
                .any(|line| line.contains("desired snapshot")),
            "{:?}",
            dry.refuses
        );
        let live = restore_cell(&archive, &dest, None, Some(&estate), false).unwrap();
        assert!(!live.writes);
        assert_eq!(
            std::fs::read_to_string(dest.join("placement-actual.json")).unwrap(),
            "sentinel\n"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
