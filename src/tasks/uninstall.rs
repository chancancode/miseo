use crate::{error::Error, spec::ToolId, workspace::Workspace};

/// Successful uninstall details for UI rendering.
#[derive(Debug, Clone)]
pub struct Outcome {
    /// Stable tool identity (`<backend>:<name>`).
    pub tool_id: ToolId,
    /// Public command links removed from `~/.miseo/.bin`.
    pub removed_commands: Vec<String>,
}

pub fn execute(workspace: &mut Workspace, tool_id: ToolId, force: bool) -> Result<Outcome, Error> {
    let removed_commands = workspace.uninstall(&tool_id, force)?;

    Ok(Outcome {
        tool_id,
        removed_commands,
    })
}
