use bevy::prelude::{Added, DetectChangesMut, Query};
use bevy_rapier3d::prelude::{Collider, RapierColliderHandle};

pub fn zone_collider_scale_fix_system(
    mut colliders: Query<&mut Collider, Added<RapierColliderHandle>>,
) {
    for mut collider in colliders.iter_mut() {
        collider.set_changed();
    }
}
