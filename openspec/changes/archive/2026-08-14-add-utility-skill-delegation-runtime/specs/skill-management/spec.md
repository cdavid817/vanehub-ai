## MODIFIED Requirements

### Requirement: Effective Skill lifecycle responses
Skill management list, preview, binding, enablement, drift, and restore responses SHALL identify canonical Skill id, effective layer, origin, type, delivery, availability, shadowed definitions, compatibility state, and delegation capability when applicable.

#### Scenario: Higher layer shadows built-in
- **WHEN** a User-layer Skill shadows a System package with the same canonical id
- **THEN** management responses SHALL identify the User definition as effective and the System definition as shadowed

#### Scenario: Supported Utility shown as delegatable
- **WHEN** an effective Utility Skill is valid and native delegated execution is supported
- **THEN** management responses SHALL identify it as available for delegation without treating it as a Role Skill

#### Scenario: Unsupported Utility shown safely
- **WHEN** a Utility Skill is present in a runtime without delegated execution support
- **THEN** management responses SHALL retain it in inventory with a runtime-specific unavailable reason rather than silently treating it as a Role Skill

