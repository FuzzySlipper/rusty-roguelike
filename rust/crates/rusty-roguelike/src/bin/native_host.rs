use std::{
    collections::BTreeSet,
    env,
    io::{self, Write},
    time::{Duration, Instant},
};

use anyhow::{bail, Context, Result};
use rusty_engine::{
    render_host_contracts::{
        RendererCameraPose, RendererPhysicalInputReadout, RendererPickFilter, RendererPickRay,
        RendererPickRequest,
    },
    render_model::{RenderHandle, RenderLayer},
    renderer_webview_host::{
        RendererResource, RendererWebviewAdapter, RendererWebviewBounds,
        RendererWebviewObservation, RendererWebviewOptions,
    },
};
use rusty_roguelike::{
    create_dungeon_frame, create_dungeon_view_composition, generate_authored_floor,
    prepare_dungeon_presentation_assets, starter_ruleset, DungeonPresentationAssets, GameSession,
    RelativeStep, SessionCommand, SessionPhase, SessionView, WorldState,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

const EXPEDITION_SEED: u64 = 5_201;
const CAMERA_POSITION: [f64; 3] = [0.0, 1.65, 0.0];
const DUNGEON_CELL_SIZE: f64 = 2.4;
const PROOF_ROUTE_TO_FIRST_ENCOUNTER: &str = "
right right right right forward forward right right forward forward
right right right right right right right right right backward
right right right right right right right right right right
right right right right right right right right right right
right right right right right right backward
right right right right right right right right right right
right right forward
";
const TORCH_CONTENT_HASH: &str =
    "sha256:49d74d297a4b7b8a271ad1299ea3a16608cb4cc460e0ea1d5a2ede36a13b5a2e";
const TORCH_BYTES: &[u8] =
    include_bytes!("../../../../../apps/app/public/assets/torch/medieval-torch.glb");

#[derive(Debug, Clone, Copy)]
struct Options {
    proof: bool,
    proof_corrupt_resource: bool,
    proof_hold: Duration,
}

impl Options {
    fn parse() -> Result<Self> {
        let mut proof = false;
        let mut proof_corrupt_resource = false;
        let mut proof_hold = Duration::ZERO;
        let mut arguments = env::args().skip(1);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--proof" => proof = true,
                "--proof-corrupt-resource" => {
                    proof = true;
                    proof_corrupt_resource = true;
                }
                "--proof-hold-ms" => {
                    let milliseconds = arguments
                        .next()
                        .context("--proof-hold-ms requires a value")?
                        .parse::<u64>()
                        .context("--proof-hold-ms must be an integer")?;
                    proof_hold = Duration::from_millis(milliseconds);
                }
                _ => bail!("unknown argument {argument}"),
            }
        }
        if !proof && !proof_hold.is_zero() {
            bail!("--proof-hold-ms requires --proof");
        }
        Ok(Self {
            proof,
            proof_corrupt_resource,
            proof_hold,
        })
    }
}

#[derive(Debug, Default)]
struct ProofEvidence {
    authority_round_trip: bool,
    camera: bool,
    frame: bool,
    input_authority: bool,
    input_noop: bool,
    pick_authority: bool,
    pick_miss: bool,
    render: bool,
    resize: bool,
    resource_rendered: bool,
    state: bool,
    views: bool,
    ready_at: Option<Instant>,
}

impl ProofEvidence {
    fn operations_complete(&self) -> bool {
        self.authority_round_trip
            && self.camera
            && self.frame
            && self.input_authority
            && self.input_noop
            && self.pick_authority
            && self.pick_miss
            && self.render
            && self.resize
            && self.resource_rendered
            && self.state
            && self.views
    }
}

#[derive(Debug, Clone, Copy)]
enum ProofPickKind {
    Miss,
    Hit { expected_entity: u64 },
}

#[derive(Debug, Clone, Copy)]
struct PendingProofPick {
    request_id: u64,
    revision_before: u64,
    kind: ProofPickKind,
}

struct NativeApplication {
    options: Options,
    window: Option<Window>,
    renderer: Option<RendererWebviewAdapter>,
    presentation_assets: DungeonPresentationAssets,
    session: GameSession,
    save_slot: Option<String>,
    retained_handles: Vec<RenderHandle>,
    published_revision: Option<u64>,
    pending_frame: Option<(u64, u64, Vec<RenderHandle>, usize)>,
    pending_input: Option<u64>,
    pending_proof_pick: Option<PendingProofPick>,
    previous_pressed_codes: BTreeSet<String>,
    previous_pointer_buttons: u16,
    next_input_poll: Instant,
    started_at: Instant,
    proof: ProofEvidence,
    dispose_request: Option<u64>,
    terminal_failure: Option<String>,
}

impl NativeApplication {
    fn new(options: Options) -> Result<Self> {
        let session = new_expedition()?;
        let presentation_assets = prepare_dungeon_presentation_assets(TORCH_BYTES)?;
        if presentation_assets.torch_source_hash != TORCH_CONTENT_HASH {
            bail!(
                "authored torch source hash drifted: expected {TORCH_CONTENT_HASH}, received {}",
                presentation_assets.torch_source_hash
            );
        }
        Ok(Self {
            options,
            window: None,
            renderer: None,
            presentation_assets,
            session,
            save_slot: None,
            retained_handles: Vec::new(),
            published_revision: None,
            pending_frame: None,
            pending_input: None,
            pending_proof_pick: None,
            previous_pressed_codes: BTreeSet::new(),
            previous_pointer_buttons: 0,
            next_input_poll: Instant::now(),
            started_at: Instant::now(),
            proof: ProofEvidence::default(),
            dispose_request: None,
            terminal_failure: None,
        })
    }

    fn mount(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let window = event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("Rusty Roguelike — preparing native renderer")
                    .with_inner_size(winit::dpi::LogicalSize::new(960, 640)),
            )
            .context("create Rusty Roguelike product window")?;
        let bounds = window_bounds(&window);
        let mut resource_bytes = self.presentation_assets.torch_resource.bytes.clone();
        if self.options.proof_corrupt_resource {
            let last = resource_bytes
                .last_mut()
                .context("packed torch resource is unexpectedly empty")?;
            *last ^= 0xff;
        }
        let resources = vec![RendererResource {
            identity: self.presentation_assets.torch_resource.resource.clone(),
            content_hash: self.presentation_assets.torch_resource.content_hash.clone(),
            media_type: "application/vnd.rusty-engine.mesh-resource".to_owned(),
            bytes: resource_bytes,
        }];
        let renderer = RendererWebviewAdapter::mount(
            &window,
            RendererWebviewOptions {
                auto_start: true,
                bounds,
                clear_color: Some(0x18130e),
                pixel_ratio: window.scale_factor(),
                resources,
            },
        )
        .map_err(|error| anyhow::anyhow!("mount Engine-owned renderer child webview: {error:?}"))?;
        self.renderer = Some(renderer);
        self.window = Some(window);
        Ok(())
    }

    fn poll_renderer(&mut self, event_loop: &ActiveEventLoop) {
        let observations = self
            .renderer
            .as_mut()
            .map(RendererWebviewAdapter::drain_observations)
            .unwrap_or_default();
        for observation in observations {
            match observation {
                Ok(observation) => self.handle_observation(observation, event_loop),
                Err(error) => {
                    self.fail(event_loop, format!("renderer observation failed: {error}"))
                }
            }
            if self.terminal_failure.is_some() {
                return;
            }
        }
    }

    fn handle_observation(
        &mut self,
        observation: RendererWebviewObservation,
        event_loop: &ActiveEventLoop,
    ) {
        match observation {
            RendererWebviewObservation::Ready(_) => {
                if self.options.proof_corrupt_resource {
                    self.fail(
                        event_loop,
                        "corrupt packed torch resource unexpectedly reached renderer ready state"
                            .to_owned(),
                    );
                    return;
                }
                self.proof.ready_at = Some(Instant::now());
                if self.options.proof {
                    println!("RUSTY_ROGUELIKE_NATIVE_READY_FOR_INPUT");
                    let _ = io::stdout().flush();
                }
                if let Err(error) = self.initialize_renderer() {
                    self.fail(event_loop, error.to_string());
                }
            }
            RendererWebviewObservation::FrameApplied {
                request_id,
                receipt,
            } => {
                if let Some((pending_id, revision, handles, torch_instances)) =
                    self.pending_frame.take()
                {
                    if pending_id != request_id {
                        self.fail(
                            event_loop,
                            format!(
                                "renderer acknowledged frame request {request_id}, expected {pending_id}"
                            ),
                        );
                        return;
                    }
                    if !receipt.applied {
                        self.fail(
                            event_loop,
                            format!("renderer rejected frame: {:?}", receipt.diagnostics),
                        );
                        return;
                    }
                    self.retained_handles = handles;
                    self.published_revision = Some(revision);
                    self.proof.frame = true;
                    self.proof.resource_rendered |= torch_instances > 0;
                    if self.options.proof
                        && self.proof.input_authority
                        && !self.proof.pick_miss
                        && self.pending_proof_pick.is_none()
                    {
                        if let Err(error) = self.request_proof_pick_miss() {
                            self.fail(event_loop, error.to_string());
                        }
                    }
                }
            }
            RendererWebviewObservation::ViewsConfigured { receipt, .. } => {
                if !receipt.applied {
                    self.fail(
                        event_loop,
                        format!("renderer rejected views: {:?}", receipt.diagnostics),
                    );
                    return;
                }
                self.proof.views = true;
            }
            RendererWebviewObservation::CameraUpdated { .. } => self.proof.camera = true,
            RendererWebviewObservation::PhysicalInputRead {
                request_id,
                readout,
            } => {
                if self.pending_input == Some(request_id) {
                    self.pending_input = None;
                    if let Err(error) = self.apply_physical_input(&readout) {
                        self.fail(event_loop, error.to_string());
                    }
                }
            }
            RendererWebviewObservation::PickCompleted {
                request_id,
                receipt,
            } => {
                if let Err(error) = self.handle_proof_pick(request_id, receipt) {
                    self.fail(event_loop, error.to_string());
                }
            }
            RendererWebviewObservation::StateRead { .. } => self.proof.state = true,
            RendererWebviewObservation::FrameRendered { .. } => self.proof.render = true,
            RendererWebviewObservation::Resized { .. } => self.proof.resize = true,
            RendererWebviewObservation::Disposed { request_id }
                if self.dispose_request == Some(request_id) =>
            {
                if self.options.proof {
                    println!(
                        "RUSTY_ROGUELIKE_NATIVE_PROOF_OK frame={} views={} camera={} resize={} resource_rendered={} input_authority={} input_noop={} pick_authority={} pick_miss={} state={} render={} authority_round_trip=true lifecycle=disposed",
                        self.proof.frame,
                        self.proof.views,
                        self.proof.camera,
                        self.proof.resize,
                        self.proof.resource_rendered,
                        self.proof.input_authority,
                        self.proof.input_noop,
                        self.proof.pick_authority,
                        self.proof.pick_miss,
                        self.proof.state,
                        self.proof.render,
                    );
                }
                event_loop.exit();
            }
            RendererWebviewObservation::MountFailed { message } => {
                self.renderer = None;
                if self.options.proof_corrupt_resource && message.contains("content hash mismatch")
                {
                    println!(
                        "RUSTY_ROGUELIKE_RESOURCE_REJECTION_OK lifecycle=transactional message={message}"
                    );
                    event_loop.exit();
                } else {
                    self.fail(
                        event_loop,
                        format!("renderer mount failed transactionally: {message}"),
                    );
                }
            }
            RendererWebviewObservation::OperationFailed {
                request_id,
                operation,
                message,
            } => self.fail(
                event_loop,
                format!("renderer operation {operation:?} request {request_id} failed: {message}"),
            ),
            _ => {}
        }
    }

    fn initialize_renderer(&mut self) -> Result<()> {
        self.publish_session()?;
        let bounds = self.current_bounds();
        {
            let renderer = self.renderer.as_mut().context("renderer is unavailable")?;
            renderer.configure_views(&create_dungeon_view_composition(1, bounds.width < 640))?;
            renderer.set_camera_pose(
                RendererCameraPose {
                    position: CAMERA_POSITION,
                    pitch_degrees: 0.0,
                    yaw_degrees: 0.0,
                },
                None,
            )?;
            renderer.read_state()?;
            renderer.render_once(None)?;
        }
        self.request_input()?;
        if self.options.proof {
            self.renderer
                .as_mut()
                .context("renderer is unavailable")?
                .resize(
                    RendererWebviewBounds {
                        width: bounds.width.saturating_sub(64).max(1),
                        height: bounds.height.saturating_sub(48).max(1),
                        ..bounds
                    },
                    self.window
                        .as_ref()
                        .context("window is unavailable")?
                        .scale_factor(),
                )?;
        }
        self.update_window_title()?;
        Ok(())
    }

    fn publish_session(&mut self) -> Result<()> {
        if self.pending_frame.is_some() {
            return Ok(());
        }
        let view = self.session.view()?;
        if self.published_revision == Some(view.revision) {
            return Ok(());
        }
        let projected = create_dungeon_frame(
            &view,
            &self.retained_handles,
            None,
            &self.presentation_assets,
        )
        .map_err(|error| anyhow::anyhow!("Rust dungeon frame is invalid: {error:?}"))?;
        let request_id = self
            .renderer
            .as_mut()
            .context("renderer is unavailable")?
            .submit_frame(&projected.frame)?;
        self.pending_frame = Some((
            request_id,
            view.revision,
            projected.handles,
            projected.torch_instance_count,
        ));
        Ok(())
    }

    fn request_input(&mut self) -> Result<()> {
        if self.pending_input.is_none() {
            self.pending_input = Some(
                self.renderer
                    .as_mut()
                    .context("renderer is unavailable")?
                    .read_physical_input()?,
            );
        }
        Ok(())
    }

    fn apply_physical_input(&mut self, input: &RendererPhysicalInputReadout) -> Result<()> {
        let pressed = input.pressed_codes.iter().cloned().collect::<BTreeSet<_>>();
        if let Some(code) = pressed
            .difference(&self.previous_pressed_codes)
            .next()
            .cloned()
        {
            let revision_before = self.session.view()?.revision;
            self.apply_key(&code)?;
            let revision_after = self.session.view()?.revision;
            if self.options.proof {
                if code == "Escape" && revision_after == revision_before {
                    self.proof.input_noop = true;
                }
                if code == "Enter" && revision_after > revision_before {
                    self.proof.input_authority = true;
                    self.prepare_proof_scene()?;
                }
            }
        }
        let left_pressed = input.pointer.buttons & 1 != 0;
        let left_was_pressed = self.previous_pointer_buttons & 1 != 0;
        if left_pressed && !left_was_pressed {
            let bounds = self.current_bounds();
            let point = [
                (input.pointer.x_pixels / f64::from(bounds.width)) * 2.0 - 1.0,
                1.0 - (input.pointer.y_pixels / f64::from(bounds.height)) * 2.0,
            ];
            self.renderer
                .as_mut()
                .context("renderer is unavailable")?
                .pick(&RendererPickRequest {
                    filter: Some(RendererPickFilter {
                        layers: vec![RenderLayer::Scene],
                        tags: vec!["enemy".to_owned()],
                        ..RendererPickFilter::default()
                    }),
                    max_distance: Some(48.0),
                    ray: RendererPickRay::Viewport { point },
                })?;
        }
        self.previous_pressed_codes = pressed;
        self.previous_pointer_buttons = input.pointer.buttons;
        Ok(())
    }

    fn prepare_proof_scene(&mut self) -> Result<()> {
        for step in PROOF_ROUTE_TO_FIRST_ENCOUNTER.split_whitespace() {
            let view = self.session.view()?;
            if !view.world.visible_actors.is_empty() {
                break;
            }
            let decision = view
                .decision
                .as_ref()
                .context("proof route requires a Rust decision")?;
            let desired = match step {
                "forward" => RelativeStep::Forward,
                "backward" => RelativeStep::Backward,
                "left" => RelativeStep::Left,
                "right" => RelativeStep::Right,
                _ => bail!("invalid proof route step {step}"),
            };
            if decision.legal_steps.contains(&desired) {
                self.session.command(SessionCommand::Step {
                    actor_entity_id: decision.actor_entity_id,
                    expected_revision: decision.expected_revision,
                    step: desired,
                })?;
                continue;
            }
            let rotations = match desired {
                RelativeStep::Forward => 0,
                RelativeStep::Right => 1,
                RelativeStep::Backward => 2,
                RelativeStep::Left => 3,
            };
            for _ in 0..rotations {
                let view = self.session.view()?;
                let decision = view
                    .decision
                    .as_ref()
                    .context("proof rotation requires a Rust decision")?;
                self.session.command(SessionCommand::TurnRight {
                    actor_entity_id: decision.actor_entity_id,
                    expected_revision: decision.expected_revision,
                })?;
            }
        }
        for _ in 0..4 {
            let view = self.session.view()?;
            if !view.world.visible_actors.is_empty()
                && !view.world.scene_placements.is_empty()
                && view.decision.as_ref().is_some_and(|decision| {
                    decision
                        .actions
                        .iter()
                        .any(|action| !action.legal_target_entity_ids.is_empty())
                })
            {
                return Ok(());
            }
            let decision = view
                .decision
                .as_ref()
                .context("proof encounter rotation requires a Rust decision")?;
            self.session.command(SessionCommand::TurnRight {
                actor_entity_id: decision.actor_entity_id,
                expected_revision: decision.expected_revision,
            })?;
        }
        bail!("proof route did not expose an authored torch and legal renderer pick target")
    }

    fn apply_key(&mut self, code: &str) -> Result<()> {
        match code {
            "F5" => self.save_slot = Some(self.session.encode_save()?),
            "F9" => {
                if let Some(encoded) = &self.save_slot {
                    self.session = GameSession::decode_save(encoded)?;
                }
            }
            "KeyN" => self.session = new_expedition()?,
            _ => {
                let view = self.session.view()?;
                if let Some(command) = command_for_key(&view, code) {
                    self.session.command(command)?;
                }
            }
        }
        self.update_window_title()?;
        Ok(())
    }

    fn use_first_legal_action(&mut self, target_entity_id: u64) -> Result<bool> {
        let view = self.session.view()?;
        let Some(decision) = view.decision else {
            return Ok(false);
        };
        let Some(action) = decision
            .actions
            .iter()
            .find(|action| action.legal_target_entity_ids.contains(&target_entity_id))
        else {
            return Ok(false);
        };
        self.session.command(SessionCommand::UseAction {
            actor_entity_id: decision.actor_entity_id,
            expected_revision: decision.expected_revision,
            action_id: action.action_id.clone(),
            target_entity_id,
        })?;
        self.update_window_title()?;
        Ok(true)
    }

    fn request_proof_pick_miss(&mut self) -> Result<()> {
        let revision_before = self.session.view()?.revision;
        let request_id = self
            .renderer
            .as_mut()
            .context("renderer is unavailable")?
            .pick(&RendererPickRequest {
                filter: Some(RendererPickFilter {
                    layers: vec![RenderLayer::Scene],
                    tags: vec!["enemy".to_owned()],
                    ..RendererPickFilter::default()
                }),
                max_distance: Some(24.0),
                ray: RendererPickRay::WorldRay {
                    origin: [1_000.0, 10.0, 1_000.0],
                    direction: [0.0, -1.0, 0.0],
                },
            })?;
        self.pending_proof_pick = Some(PendingProofPick {
            request_id,
            revision_before,
            kind: ProofPickKind::Miss,
        });
        Ok(())
    }

    fn request_proof_pick_hit(&mut self) -> Result<()> {
        let view = self.session.view()?;
        let decision = view
            .decision
            .as_ref()
            .context("proof pick requires an active Rust decision")?;
        let expected_entity = decision
            .actions
            .iter()
            .flat_map(|action| action.legal_target_entity_ids.iter().copied())
            .next()
            .context("proof pick requires a legal target")?;
        let target = view
            .world
            .visible_actors
            .iter()
            .find(|actor| actor.entity_id == expected_entity)
            .context("legal proof target is not visible")?;
        let request_id = self
            .renderer
            .as_mut()
            .context("renderer is unavailable")?
            .pick(&RendererPickRequest {
                filter: Some(RendererPickFilter {
                    layers: vec![RenderLayer::Scene],
                    tags: vec!["enemy".to_owned()],
                    ..RendererPickFilter::default()
                }),
                max_distance: Some(24.0),
                ray: RendererPickRay::WorldRay {
                    origin: [
                        f64::from(target.lateral) * DUNGEON_CELL_SIZE,
                        10.0,
                        -f64::from(target.depth) * DUNGEON_CELL_SIZE,
                    ],
                    direction: [0.0, -1.0, 0.0],
                },
            })?;
        self.pending_proof_pick = Some(PendingProofPick {
            request_id,
            revision_before: view.revision,
            kind: ProofPickKind::Hit { expected_entity },
        });
        Ok(())
    }

    fn handle_proof_pick(
        &mut self,
        request_id: u64,
        receipt: rusty_engine::render_host_contracts::RendererPickReceipt,
    ) -> Result<()> {
        if !self.options.proof {
            if let Some(entity) = receipt
                .hint
                .and_then(|hint| hint.source_trace)
                .map(|trace| trace.entity)
            {
                self.use_first_legal_action(entity)?;
            }
            return Ok(());
        }
        let pending = self
            .pending_proof_pick
            .take()
            .context("renderer returned an unrequested proof pick")?;
        if pending.request_id != request_id {
            bail!(
                "renderer acknowledged proof pick request {request_id}, expected {}",
                pending.request_id
            );
        }
        match pending.kind {
            ProofPickKind::Miss => {
                if receipt.hint.is_some()
                    || self.session.view()?.revision != pending.revision_before
                {
                    bail!("proof miss pick unexpectedly hit or changed Rust authority");
                }
                self.proof.pick_miss = true;
                self.request_proof_pick_hit()?;
            }
            ProofPickKind::Hit { expected_entity } => {
                let actual_entity = receipt
                    .hint
                    .and_then(|hint| hint.source_trace)
                    .map(|trace| trace.entity)
                    .context("proof hit pick returned no entity source trace")?;
                if actual_entity != expected_entity {
                    bail!(
                        "proof hit pick resolved entity {actual_entity}, expected {expected_entity}"
                    );
                }
                if !self.use_first_legal_action(actual_entity)?
                    || self.session.view()?.revision <= pending.revision_before
                {
                    bail!("proof hit pick did not advance Rust gameplay authority");
                }
                self.proof.pick_authority = true;
                self.exercise_authority_round_trip()?;
            }
        }
        Ok(())
    }

    fn exercise_authority_round_trip(&mut self) -> Result<()> {
        let saved_view = self.session.view()?;
        let encoded = self.session.encode_save()?;
        let command = saved_view
            .decision
            .as_ref()
            .and_then(|decision| {
                decision
                    .legal_steps
                    .first()
                    .copied()
                    .map(|step| SessionCommand::Step {
                        actor_entity_id: decision.actor_entity_id,
                        expected_revision: decision.expected_revision,
                        step,
                    })
            })
            .or_else(|| {
                saved_view
                    .decision
                    .as_ref()
                    .map(|decision| SessionCommand::Wait {
                        actor_entity_id: decision.actor_entity_id,
                        expected_revision: decision.expected_revision,
                    })
            })
            .context("native proof has no legal expedition command")?;
        let changed = self.session.command(command)?;
        if changed.revision <= saved_view.revision {
            bail!("native proof command did not advance authoritative revision");
        }
        self.session = GameSession::decode_save(&encoded)?;
        if self.session.view()? != saved_view {
            bail!("native proof save/load did not restore the exact Rust view");
        }
        self.save_slot = Some(encoded);
        self.proof.authority_round_trip = true;
        Ok(())
    }

    fn update_window_title(&self) -> Result<()> {
        let view = self.session.view()?;
        // The renderer is a native X11 child window. Some WebKit/GLX stacks
        // invalidate title mutation on the parent after child attachment, so
        // the stable product title is selected before mount and live state is
        // carried by Rust rather than mirrored into window-manager metadata.
        let _ = (view.round, view.revision);
        Ok(())
    }

    fn current_bounds(&self) -> RendererWebviewBounds {
        self.window.as_ref().map(window_bounds).unwrap_or_default()
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        self.renderer = None;
        self.terminal_failure = Some(message);
        event_loop.exit();
    }

    fn maybe_finish_proof(&mut self, event_loop: &ActiveEventLoop) {
        if !self.options.proof
            || !self.proof.operations_complete()
            || self.dispose_request.is_some()
        {
            return;
        }
        let Some(ready_at) = self.proof.ready_at else {
            return;
        };
        if ready_at.elapsed() < self.options.proof_hold {
            return;
        }
        match self
            .renderer
            .as_mut()
            .context("renderer disappeared before proof disposal")
            .and_then(|renderer| renderer.dispose().map_err(Into::into))
        {
            Ok(request_id) => self.dispose_request = Some(request_id),
            Err(error) => self.fail(event_loop, error.to_string()),
        }
    }
}

impl ApplicationHandler for NativeApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            if let Err(error) = self.mount(event_loop) {
                let message = error.to_string();
                if self.options.proof_corrupt_resource
                    && message.contains("resource bytes do not match the declared SHA-256 identity")
                {
                    println!(
                        "RUSTY_ROGUELIKE_RESOURCE_REJECTION_OK lifecycle=transactional message={message}"
                    );
                    event_loop.exit();
                } else {
                    self.fail(event_loop, message);
                }
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                if self.dispose_request.is_none() {
                    match self.renderer.as_mut().map(RendererWebviewAdapter::dispose) {
                        Some(Ok(request_id)) => self.dispose_request = Some(request_id),
                        Some(Err(error)) => self.fail(event_loop, error.to_string()),
                        None => event_loop.exit(),
                    }
                }
            }
            WindowEvent::Resized(_) if self.proof.ready_at.is_some() => {
                let bounds = self.current_bounds();
                let pixel_ratio = self.window.as_ref().map_or(1.0, Window::scale_factor);
                if let Some(renderer) = self.renderer.as_mut() {
                    if let Err(error) = renderer.resize(bounds, pixel_ratio) {
                        self.fail(event_loop, error.to_string());
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "linux")]
        while gtk::events_pending() {
            gtk::main_iteration_do(false);
        }

        if self.started_at.elapsed() > Duration::from_secs(30) && self.options.proof {
            self.fail(event_loop, "native renderer proof timed out".to_owned());
            return;
        }
        self.poll_renderer(event_loop);
        if self.terminal_failure.is_some() {
            return;
        }
        if self.proof.ready_at.is_some() && self.dispose_request.is_none() {
            if let Err(error) = self.publish_session() {
                self.fail(event_loop, error.to_string());
                return;
            }
            if Instant::now() >= self.next_input_poll {
                if let Err(error) = self.request_input() {
                    self.fail(event_loop, error.to_string());
                    return;
                }
                self.next_input_poll = Instant::now() + Duration::from_millis(40);
            }
        }
        self.maybe_finish_proof(event_loop);
    }
}

fn command_for_key(view: &SessionView, code: &str) -> Option<SessionCommand> {
    if view.phase == SessionPhase::Preparation && code == "Enter" {
        return Some(SessionCommand::BeginExpedition {
            expected_revision: view.revision,
        });
    }
    let decision = view.decision.as_ref()?;
    let actor_entity_id = decision.actor_entity_id;
    let expected_revision = decision.expected_revision;
    match code {
        "KeyW" | "ArrowUp" => Some(SessionCommand::Step {
            actor_entity_id,
            expected_revision,
            step: RelativeStep::Forward,
        }),
        "KeyS" | "ArrowDown" => Some(SessionCommand::Step {
            actor_entity_id,
            expected_revision,
            step: RelativeStep::Backward,
        }),
        "KeyA" => Some(SessionCommand::Step {
            actor_entity_id,
            expected_revision,
            step: RelativeStep::Left,
        }),
        "KeyD" => Some(SessionCommand::Step {
            actor_entity_id,
            expected_revision,
            step: RelativeStep::Right,
        }),
        "KeyQ" | "ArrowLeft" => Some(SessionCommand::TurnLeft {
            actor_entity_id,
            expected_revision,
        }),
        "KeyE" | "ArrowRight" => Some(SessionCommand::TurnRight {
            actor_entity_id,
            expected_revision,
        }),
        "Space" if decision.can_wait => Some(SessionCommand::Wait {
            actor_entity_id,
            expected_revision,
        }),
        _ => None,
    }
}

fn window_bounds(window: &Window) -> RendererWebviewBounds {
    let size = window.inner_size();
    let scale = window.scale_factor();
    RendererWebviewBounds {
        x: 0,
        y: 0,
        width: ((f64::from(size.width) / scale).round() as u32).max(1),
        height: ((f64::from(size.height) / scale).round() as u32).max(1),
    }
}

fn new_expedition() -> Result<GameSession> {
    Ok(GameSession::new(WorldState::new(
        generate_authored_floor(EXPEDITION_SEED)?,
        starter_ruleset()?,
    )?)?)
}

fn main() -> Result<()> {
    #[cfg(target_os = "linux")]
    gtk::init().context("initialize GTK for the native renderer host")?;
    let options = Options::parse()?;
    let event_loop = EventLoop::new().context("create native product event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut application = NativeApplication::new(options)?;
    event_loop
        .run_app(&mut application)
        .context("run native product event loop")?;
    if let Some(message) = application.terminal_failure {
        bail!(message);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_key_mapping_emits_only_revision_bound_semantic_commands() {
        let mut session = new_expedition().unwrap();
        let preparation = session.view().unwrap();
        assert!(matches!(
            command_for_key(&preparation, "Enter"),
            Some(SessionCommand::BeginExpedition { .. })
        ));
        session
            .command(command_for_key(&preparation, "Enter").unwrap())
            .unwrap();
        let expedition = session.view().unwrap();
        let decision = expedition.decision.as_ref().unwrap();
        assert_eq!(
            command_for_key(&expedition, "KeyQ"),
            Some(SessionCommand::TurnLeft {
                actor_entity_id: decision.actor_entity_id,
                expected_revision: decision.expected_revision,
            })
        );
        assert!(command_for_key(&expedition, "Escape").is_none());
    }
}
