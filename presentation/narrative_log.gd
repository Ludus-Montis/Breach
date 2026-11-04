class_name NarrativeLog
extends RefCounted
## Pure mapping from a suspicion tier to a log line, per
## specs/05-presentation-and-hud.md and Decision 8: narrative signal only, never the
## raw meter (no Scout exists yet to justify showing it). Calm produces no message -
## the caller skips appending an empty string rather than logging "nothing happened."
##
## The tiers are the crate's (`kingdom::Tier`, native/rust/src/kingdom.rs, spec 39 step
## 9b-iii) by number, as the lane's binding emits them in `suspicion_tier_changed`.

enum Tier { CALM, WARY, ALARMED, MOBILIZED, FULL_ALERT }


static func message_for_tier(tier: int) -> String:
	match tier:
		Tier.WARY:
			return "Something feels off nearby."
		Tier.ALARMED:
			return "The defenders are on alert."
		Tier.MOBILIZED:
			return "A Hero Party is marching toward the fighting."
		Tier.FULL_ALERT:
			return "The defenders are fully mobilized."
		_:
			return ""
