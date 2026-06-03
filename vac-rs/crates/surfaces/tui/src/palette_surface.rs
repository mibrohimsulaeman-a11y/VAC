// Top-level palette surface action declarations.
//
// This catalog keeps the product-facing palette action names in Rust so tests
// can guard `.vac/surfaces/palette.yaml` against drift. The actual UI entry
// points may be slash commands, popups, or dashboard routes, but each action
// here must have route metadata in the palette surface manifest.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PaletteSurfaceAction {
    pub(crate) action: &'static str,
}

pub(crate) const TOP_LEVEL_PALETTE_ACTIONS: &[PaletteSurfaceAction] = &[
    PaletteSurfaceAction {
        action: "open_capabilities",
    },
    PaletteSurfaceAction {
        action: "open_workflow",
    },
    PaletteSurfaceAction {
        action: "open_status",
    },
    PaletteSurfaceAction {
        action: "open_activity",
    },
    PaletteSurfaceAction {
        action: "open_debug_config",
    },
    PaletteSurfaceAction {
        action: "open_model",
    },
];

#[cfg(test)]
mod tests {
    use super::TOP_LEVEL_PALETTE_ACTIONS;
    use crate::surface_route_catalog::SurfaceRouteCatalog;

    #[test]
    fn top_level_palette_actions_are_declared_in_palette_surface_manifest() {
        let root = std::env::current_dir().expect("current dir");
        let catalog = SurfaceRouteCatalog::load(root);
        let missing = TOP_LEVEL_PALETTE_ACTIONS
            .iter()
            .filter_map(|entry| {
                catalog
                    .palette_route(entry.action)
                    .is_none()
                    .then_some(entry.action)
            })
            .collect::<Vec<_>>();

        assert!(
            missing.is_empty(),
            "missing palette routes in .vac/surfaces/palette.yaml: {}",
            missing.join(", ")
        );
    }

    #[test]
    fn palette_surface_manifest_does_not_declare_unknown_top_level_actions() {
        let root = std::env::current_dir().expect("current dir");
        let catalog = SurfaceRouteCatalog::load(root);
        let known = TOP_LEVEL_PALETTE_ACTIONS
            .iter()
            .map(|entry| entry.action)
            .collect::<std::collections::BTreeSet<_>>();
        let unknown = catalog
            .palette_routes()
            .into_iter()
            .filter_map(|(action, route)| {
                (route.visible && !known.contains(action.as_str())).then_some(action)
            })
            .collect::<Vec<_>>();

        assert!(
            unknown.is_empty(),
            "unknown visible palette routes in .vac/surfaces/palette.yaml: {}",
            unknown.join(", ")
        );
    }
}
