//! Lifecycle tasks shared by CLI command handlers.

use crate::{
    error::Error,
    workspace::{InstallPlan, Workspace},
};

pub mod install;
pub mod uninstall;
pub mod upgrade;

fn with_new_tool_cleanup<T>(
    workspace: &Workspace,
    plan: &InstallPlan,
    result: Result<T, Error>,
) -> Result<T, Error> {
    match result {
        Ok(value) => Ok(value),
        Err(err) => {
            workspace.cleanup_plan(plan);
            Err(err)
        }
    }
}
