use bevy_ecs::system::Resource;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::AdapterInfo;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Resource)]
pub enum XrEnvironmentBlendMode {
    Opaque,
    AlphaBlend,
    Additive,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Resource)]
pub enum XrInteractionMode {
    ScreenSpace,
    WorldSpace,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, Resource)]
pub enum XrVisibilityState {
    VisibleFocused,
    VisibleUnfocused,
    Hidden,
}

#[derive(Resource)]
pub struct XrGraphicsContext {
    pub instance: wgpu::Instance,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub adapter_info: AdapterInfo,
}

// Trait implemented by XR backends that support display mode.
pub trait XrPresentationSession: Send + Sync + 'static {
    fn get_swapchains(&mut self) -> Vec<Vec<u64>>;
}
