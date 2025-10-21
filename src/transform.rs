use bevy::picking::prelude::*;
use bevy::prelude::*;
use bevy_mod_imgui::prelude::*;
use bevy_rapier3d::prelude::*;

// ============================================================================
// COMPONENTS & RESOURCES
// ============================================================================

#[derive(Component)]
pub struct Selected;

#[derive(Resource, Default)]
pub struct TransformGizmoState {
    pub selected_entity: Option<Entity>,
    pub mode: TransformMode,
    pub is_dragging: bool,
    pub drag_start_pos: Vec2,
    pub initial_transform: Transform,
}

#[derive(Default, PartialEq, Clone, Copy)]
pub enum TransformMode {
    #[default]
    Translate,
    Rotate,
    Scale,
}

fn handle_selection(
    mut commands: Commands,
    mut gizmo_state: ResMut<TransformGizmoState>,
    mut click_events: EventReader<Pointer<Click>>,
    pickable_query: Query<Entity, With<Pickable>>,
    selected_query: Query<Entity, With<Selected>>,
) {
    for click in click_events.read() {
        let entity = click.target;
        if pickable_query.get(entity).is_ok() {
            for sel in selected_query.iter() {
                commands.entity(sel).remove::<Selected>();
            }
            commands.entity(entity).insert(Selected);
            gizmo_state.selected_entity = Some(entity);
            info!("Selezionato: {:?}", entity);
        }
    }
}

fn handle_deselection(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    selected_query: Query<Entity, With<Selected>>,
    mut gizmo_state: ResMut<TransformGizmoState>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        for entity in selected_query.iter() {
            commands.entity(entity).remove::<Selected>();
        }
        gizmo_state.selected_entity = None;
        info!("Deselezione");
    }
}


fn handle_transform(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut gizmo_state: ResMut<TransformGizmoState>,
    windows: Query<&Window>,
    mut selected_query: Query<&mut Transform, With<Selected>>,
) {
    if keyboard.just_pressed(KeyCode::KeyG) {
        gizmo_state.mode = TransformMode::Translate;
        info!("Modalità: Translate");
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        gizmo_state.mode = TransformMode::Rotate;
        info!("Modalità: Rotate");
    }
    if keyboard.just_pressed(KeyCode::KeyS) {
        gizmo_state.mode = TransformMode::Scale;
        info!("Modalità: Scale");
    }

    if gizmo_state.selected_entity.is_none() {
        return;
    }

    let Ok(mut transform) = selected_query.single_mut() else {
        return;
    };

    if mouse_button.just_pressed(MouseButton::Right) {
        gizmo_state.is_dragging = true;
        gizmo_state.initial_transform = *transform;
        if let Ok(window) = windows.single() {
            if let Some(cursor_pos) = window.cursor_position() {
                gizmo_state.drag_start_pos = cursor_pos;
            }
        }
    }

    if mouse_button.just_released(MouseButton::Right) {
        gizmo_state.is_dragging = false;
    }

    if gizmo_state.is_dragging {
        if let Ok(window) = windows.single() {
            if let Some(cursor_pos) = window.cursor_position() {
                let delta = cursor_pos - gizmo_state.drag_start_pos;
                match gizmo_state.mode {
                    TransformMode::Translate => {
                        let speed = 0.01;
                        transform.translation.x =
                            gizmo_state.initial_transform.translation.x + delta.x * speed;
                        transform.translation.z =
                            gizmo_state.initial_transform.translation.z - delta.y * speed;
                    }
                    TransformMode::Rotate => {
                        let speed = 0.01;
                        transform.rotation = gizmo_state.initial_transform.rotation
                            * Quat::from_rotation_y(delta.x * speed);
                    }
                    TransformMode::Scale => {
                        let speed = 0.01;
                        let scale_factor = 1.0 + delta.y * speed;
                        transform.scale = gizmo_state.initial_transform.scale * scale_factor;
                    }
                }
            }
        }
    }
}

// ============================================================================
// ENTITY MANIPULATION SYSTEMS
// ============================================================================

fn handle_duplication(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    selected_query: Query<
        (
            Entity,
            &Transform,
            Option<&Mesh3d>,
            Option<&MeshMaterial3d<StandardMaterial>>,
        ),
        With<Selected>>
) {
    if keyboard.pressed(KeyCode::ShiftLeft) && keyboard.just_pressed(KeyCode::KeyD) {
        for (entity, transform, mesh, material) in selected_query.iter() {
            let mut new_transform = *transform;
            new_transform.translation.x += 2.0;

            let mut entity_commands =
                commands.spawn((new_transform, Pickable::default(), RapierPickable));

            if let Some(mesh) = mesh {
                entity_commands.insert(Mesh3d(mesh.0.clone()));
            }
            if let Some(material) = material {
                entity_commands.insert(MeshMaterial3d(material.0.clone()));
            }

            info!("Duplicato: {:?}", entity);
        }
    }
}

fn handle_deletion(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    selected_query: Query<Entity, With<Selected>>,
    mut gizmo_state: ResMut<TransformGizmoState>,
) {
    if keyboard.just_pressed(KeyCode::Delete) || keyboard.just_pressed(KeyCode::KeyX) {
        for entity in selected_query.iter() {
            commands.entity(entity).despawn();
            info!("Eliminato: {:?}", entity);
        }
        gizmo_state.selected_entity = None;
    }
}

// ============================================================================
// VISUAL FEEDBACK
// ============================================================================

fn draw_selection_outline(
    mut gizmos: Gizmos,
    selected_query: Query<&GlobalTransform, With<Selected>>,
) {
    for global_transform in selected_query.iter() {
        let pos = global_transform.translation();
        gizmos.cuboid(
            Transform::from_translation(pos).with_scale(Vec3::splat(1.1)),
            Color::srgb(1.0, 1.0, 0.0),
        );

        let size = 2.0;
        gizmos.line(
            pos,
            pos + global_transform.right() * size,
            Color::srgb(1.0, 0.0, 0.0),
        );
        gizmos.line(
            pos,
            pos + global_transform.up() * size,
            Color::srgb(0.0, 1.0, 0.0),
        );
        gizmos.line(
            pos,
            pos + global_transform.forward() * size,
            Color::srgb(0.0, 0.0, 1.0),
        );
    }
}

// ============================================================================
// UI SYSTEMS
// ============================================================================

fn gizmo_controls_ui(
    mut context: NonSendMut<ImguiContext>,
    mut gizmo_state: ResMut<TransformGizmoState>,
    mut selected_query: Query<&mut Transform, With<Selected>>,
) {
    let ui = context.ui();
    let window = ui.window("Transform Gizmo");
    window
        .position([900.0, 300.0], imgui::Condition::FirstUseEver)
        .size([320.0, 500.0], imgui::Condition::FirstUseEver)
        .build(|| {
            ui.text("Transform Controls");
            ui.separator();

            if let Some(entity) = gizmo_state.selected_entity {
                ui.text(format!("Selected: {:?}", entity));
            } else {
                ui.text_colored([0.7, 0.7, 0.7, 1.0], "No entity selected");
            }

            ui.separator();

            if ui.collapsing_header("Transform Mode", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                let mut current_mode = gizmo_state.mode;
                if ui.radio_button("Translate (G)", &mut current_mode, TransformMode::Translate) {
                    gizmo_state.mode = TransformMode::Translate;
                }
                if ui.radio_button("Rotate (R)", &mut current_mode, TransformMode::Rotate) {
                    gizmo_state.mode = TransformMode::Rotate;
                }
                if ui.radio_button("Scale (S)", &mut current_mode, TransformMode::Scale) {
                    gizmo_state.mode = TransformMode::Scale;
                }
            }

            if gizmo_state.selected_entity.is_some() {
                if let Ok(mut transform) = selected_query.single_mut() {
                    if ui.collapsing_header("Position", imgui::TreeNodeFlags::DEFAULT_OPEN) {
                        let mut pos = [
                            transform.translation.x,
                            transform.translation.y,
                            transform.translation.z,
                        ];
                        if ui.input_float3("Position", &mut pos).build() {
                            transform.translation = Vec3::new(pos[0], pos[1], pos[2]);
                        }
                        if ui.slider("PosX", -100.0, 100.0, &mut pos[0]) {
                            transform.translation = Vec3::new(pos[0], pos[1], pos[2]);
                        }
                        if ui.slider("PosY", -100.0, 100.0, &mut pos[1]) {
                            transform.translation = Vec3::new(pos[0], pos[1], pos[2]);
                        }
                        if ui.slider("PosZ", -100.0, 100.0, &mut pos[2]) {
                            transform.translation = Vec3::new(pos[0], pos[1], pos[2]);
                        }
                    }

                    if ui.collapsing_header("Rotation", imgui::TreeNodeFlags::empty()) {
                        let (mut x, mut y, mut z) = transform.rotation.to_euler(EulerRot::XYZ);
                        x = x.to_degrees();
                        y = y.to_degrees();
                        z = z.to_degrees();
                        let mut euler = [x, y, z];

                        // Input fields
                        if ui.input_float3("Rotation (deg)", &mut euler).build() {
                            // Normalizza gli angoli nel range [-180, 180]
                            for angle in &mut euler {
                                *angle = (*angle + 180.0).rem_euclid(360.0) - 180.0;
                            }
                            transform.rotation = Quat::from_euler(
                                EulerRot::XYZ,
                                euler[0].to_radians(),
                                euler[1].to_radians(),
                                euler[2].to_radians(),
                            );
                        }

                        // Sliders individuali
                        if ui.slider("RotX", -180.0, 180.0, &mut euler[0]) {
                            transform.rotation = Quat::from_euler(
                                EulerRot::XYZ,
                                euler[0].to_radians(),
                                euler[1].to_radians(),
                                euler[2].to_radians(),
                            );
                        }
                        if ui.slider("RotY", -90.0, 90.0, &mut euler[1]) {
                            transform.rotation = Quat::from_euler(
                                EulerRot::XYZ,
                                euler[0].to_radians(),
                                euler[1].to_radians(),
                                euler[2].to_radians(),
                            );
                        }
                        if ui.slider("RotZ", -180.0, 180.0, &mut euler[2]) {
                            transform.rotation = Quat::from_euler(
                                EulerRot::XYZ,
                                euler[0].to_radians(),
                                euler[1].to_radians(),
                                euler[2].to_radians(),
                            );
                        }

                        ui.separator();

                        // Bottoni Flip
                        ui.text("Flip Axes:");
                        ui.same_line();
                        if ui.button("Flip X") {
                            euler[0] = (euler[0] + 180.0).rem_euclid(360.0) - 180.0;
                            transform.rotation = Quat::from_euler(
                                EulerRot::XYZ,
                                euler[0].to_radians(),
                                euler[1].to_radians(),
                                euler[2].to_radians(),
                            );
                        }
                        ui.same_line();
                        if ui.button("Flip Y") {
                            euler[1] = (euler[1] + 180.0).rem_euclid(360.0) - 180.0;
                            transform.rotation = Quat::from_euler(
                                EulerRot::XYZ,
                                euler[0].to_radians(),
                                euler[1].to_radians(),
                                euler[2].to_radians(),
                            );
                        }
                        ui.same_line();
                        if ui.button("Flip Z") {
                            euler[2] = (euler[2] + 180.0).rem_euclid(360.0) - 180.0;
                            transform.rotation = Quat::from_euler(
                                EulerRot::XYZ,
                                euler[0].to_radians(),
                                euler[1].to_radians(),
                                euler[2].to_radians(),
                            );
                        }

                        ui.separator();

                        // Bottoni Preset
                        ui.text("Rotation Presets:");
                        if ui.button("Reset##rot") {
                            transform.rotation = Quat::IDENTITY;
                        }
                        ui.same_line();
                        if ui.button("90° X") {
                            transform.rotation =
                                Quat::from_euler(EulerRot::XYZ, 90.0_f32.to_radians(), 0.0, 0.0);
                        }
                        ui.same_line();
                        if ui.button("90° Y") {
                            transform.rotation =
                                Quat::from_euler(EulerRot::XYZ, 0.0, 90.0_f32.to_radians(), 0.0);
                        }
                        ui.same_line();
                        if ui.button("90° Z") {
                            transform.rotation =
                                Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, 90.0_f32.to_radians());
                        }

                        // Seconda riga di preset
                        if ui.button("180° X") {
                            transform.rotation =
                                Quat::from_euler(EulerRot::XYZ, 180.0_f32.to_radians(), 0.0, 0.0);
                        }
                        ui.same_line();
                        if ui.button("180° Y") {
                            transform.rotation =
                                Quat::from_euler(EulerRot::XYZ, 0.0, 180.0_f32.to_radians(), 0.0);
                        }
                        ui.same_line();
                        if ui.button("180° Z") {
                            transform.rotation =
                                Quat::from_euler(EulerRot::XYZ, 0.0, 0.0, 180.0_f32.to_radians());
                        }
                        ui.same_line();
                        if ui.button("-90° X") {
                            transform.rotation =
                                Quat::from_euler(EulerRot::XYZ, -90.0_f32.to_radians(), 0.0, 0.0);
                        }
                    }

                    if ui.collapsing_header("Scale", imgui::TreeNodeFlags::empty()) {
                        let mut scale = [transform.scale.x, transform.scale.y, transform.scale.z];
                        if ui.input_float3("Scale", &mut scale).build() {
                            transform.scale = Vec3::new(scale[0], scale[1], scale[2]);
                        }
                        let mut uniform_scale = transform.scale.x;
                        if ui.slider("Uniform Scale", 0.1, 5.0, &mut uniform_scale) {
                            transform.scale = Vec3::splat(uniform_scale);
                        }
                    }

                    ui.separator();

                    if ui.button("Reset Position") {
                        transform.translation = Vec3::ZERO;
                    }
                    ui.same_line();
                    if ui.button("Reset Rotation") {
                        transform.rotation = Quat::IDENTITY;
                    }
                    if ui.button("Reset Scale") {
                        transform.scale = Vec3::ONE;
                    }
                    ui.same_line();
                    if ui.button("Reset All") {
                        *transform = Transform::default();
                    }
                }
            }

            ui.separator();

            if ui.collapsing_header("Keyboard Shortcuts", imgui::TreeNodeFlags::empty()) {
                ui.bullet_text("Left Click: Select");
                ui.bullet_text("Right Click + Drag: Transform");
                ui.bullet_text("G: Translate mode");
                ui.bullet_text("R: Rotate mode");
                ui.bullet_text("S: Scale mode");
                ui.bullet_text("Shift+D: Duplicate");
                ui.bullet_text("X/Delete: Delete");
                ui.bullet_text("ESC: Deselect");
            }
        });
}

fn entity_list_ui(
    mut context: NonSendMut<ImguiContext>,
    mut commands: Commands,
    pickable_query: Query<(Entity, Option<&Name>, &Transform), With<Pickable>>,
    selected_query: Query<Entity, With<Selected>>,
    mut gizmo_state: ResMut<TransformGizmoState>,
) {
    let ui = context.ui();
    let window = ui.window("Entity List");
    window
        .position([10.0, 810.0], imgui::Condition::FirstUseEver)
        .size([320.0, 250.0], imgui::Condition::FirstUseEver)
        .build(|| {
            ui.text(format!(
                "Pickable Entities: {}",
                pickable_query.iter().count()
            ));
            ui.separator();

            for (entity, name, transform) in pickable_query.iter() {
                let is_selected = selected_query.contains(entity);
                let label = if let Some(name) = name {
                    format!("{} ({:?})", name.as_str(), entity)
                } else {
                    format!("Entity {:?}", entity)
                };

                if is_selected {
                    ui.text_colored([1.0, 1.0, 0.0, 1.0], &label);
                } else {
                    ui.text(&label);
                }

                ui.same_line();
                let select_label = format!("Select###{:?}", entity);
                if ui.small_button(&select_label) {
                    for selected in selected_query.iter() {
                        commands.entity(selected).remove::<Selected>();
                    }
                    commands.entity(entity).insert(Selected);
                    gizmo_state.selected_entity = Some(entity);
                }

                ui.same_line();
                let delete_label = format!("X###{:?}", entity);
                if ui.small_button(&delete_label) {
                    commands.entity(entity).despawn();
                    if gizmo_state.selected_entity == Some(entity) {
                        gizmo_state.selected_entity = None;
                    }
                }

                ui.text_disabled(format!(
                    "  Pos: [{:.1}, {:.1}, {:.1}]",
                    transform.translation.x, transform.translation.y, transform.translation.z,
                ));
            }
        });
}

pub struct TransformGizmoPlugin;

impl Plugin for TransformGizmoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransformGizmoState>()
            .add_systems(Update, handle_selection)
            .add_systems(Update, handle_transform)
            .add_systems(Update, handle_deselection)
            .add_systems(Update, handle_duplication)
            .add_systems(Update, handle_deletion)
            .add_systems(Update, draw_selection_outline)
            .add_systems(Update, gizmo_controls_ui)
            .add_systems(Update, entity_list_ui);
    }
}


pub trait PickableExt {
    fn with_pickable(self) -> Self;
}

impl PickableExt for EntityCommands<'_> {
    fn with_pickable(mut self) -> Self {
        self.insert(Pickable::default());
        self.insert(RapierPickable);
        self
    }
}