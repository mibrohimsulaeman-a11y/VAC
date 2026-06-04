mod discoverable;
mod injection;
pub(crate) mod mentions;
mod render;
#[cfg(test)]
pub(crate) mod test_support;

pub(crate) use vac_plugin::PluginCapabilitySummary;

pub(crate) use discoverable::list_tool_suggest_discoverable_plugins;
pub(crate) use injection::build_plugin_injections;
pub(crate) use render::render_explicit_plugin_instructions;
