//! Says so, once, when a component arrives with settings that cannot work.
//!
//! Each settings component has a `problems()`; a host that reads fires from a file, a panel or BRP and
//! never calls it gets a dark light or a flame that draws nothing and no line in the log. An observer
//! per component calls it on `Add` and warns. Nothing is refused or clamped.

use bevy::prelude::*;

use crate::Problem;

/// A component whose settings can be checked.
pub(crate) trait Checked: Component {
    /// The type's name as the warning spells it.
    const NAME: &'static str;
    /// What is unusable in this value.
    fn check(&self) -> Vec<Problem>;
}

macro_rules! checked {
    ($($t:ty),+ $(,)?) => {$(
        impl Checked for $t {
            const NAME: &'static str = stringify!($t);
            fn check(&self) -> Vec<Problem> {
                self.problems()
            }
        }
    )+};
}

checked!(crate::Flicker, crate::Snuff, crate::FlameSite, crate::FlameEmbers, crate::PoolCandidate);

/// Warns about the first bad `C` this observer sees, then stays quiet: a scene of a thousand identical
/// bad torches is one line, and a different bad component type has its own observer.
fn warn_on_add<C: Checked>(add: On<Add<C>>, components: Query<&C>, mut warned: Local<bool>) {
    if *warned {
        return;
    }
    let Ok(component) = components.get(add.entity) else {
        return;
    };
    let problems = component.check();
    if problems.is_empty() {
        return;
    }
    *warned = true;
    let list = problems.iter().map(ToString::to_string).collect::<Vec<_>>().join("; ");
    warn!(
        "bevy_lights: {} on {} has unusable settings ({} problem(s); further ones are not reported): {list}",
        C::NAME,
        add.entity,
        problems.len()
    );
}

/// Checks every `C` as it is added.
pub(crate) fn watch<C: Checked>(app: &mut App) {
    app.add_observer(warn_on_add::<C>);
}
