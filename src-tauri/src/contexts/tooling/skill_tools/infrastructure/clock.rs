use crate::contexts::tooling::skill_tools::application::SkillToolClockPort;
use crate::platform::clock::SystemClock;

pub(crate) struct SystemSkillToolClock;

impl SkillToolClockPort for SystemSkillToolClock {
    fn now(&self) -> String {
        SystemClock.rfc3339()
    }
}
