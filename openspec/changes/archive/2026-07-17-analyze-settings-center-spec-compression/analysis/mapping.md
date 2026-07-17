# Requirement Mapping

| Source lines | Target capability | Preservation rule |
| --- | --- | --- |
| 6-178 | `settings-center-ui` | Retain shared shell, UCD styles, common data orchestration, navigation, scrolling, and form validation. |
| 182-312 | `settings-cli-management-ui` | Move intact, including every scenario. |
| 316-365 and 499-518 | `settings-basic-configuration-ui` | Move intact, including proxy and log-management scenarios. |
| 384-495 | `settings-skill-management-ui` | Move intact, including dialogs and drift scenarios. |
| 522-584 | `settings-center-ui` | Retain shared visual-system and settings-wide localization requirements. |
| 589-622 | `settings-usage-statistics-ui` | Move intact. |
| 626-666 | `settings-extension-management-ui` | Move intact. |
| 670-751 | `settings-im-management-ui` | Move intact. |
| 754-773 | `settings-floating-assistant-ui` | Move intact. |

Every range is a move, not a deletion. The implementation change must copy complete Requirement blocks, including all `#### Scenario` headings, to its target spec before removing them from the source spec.
