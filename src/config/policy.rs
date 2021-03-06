use lib_lxd::{LxdInstance, LxdInstanceName, LxdInstanceStatus, LxdProject, LxdProjectName};
use serde::Deserialize;
use std::collections::HashSet;
use std::hash::Hash;

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct Policy {
    pub included_projects: Option<HashSet<LxdProjectName>>,
    pub excluded_projects: Option<HashSet<LxdProjectName>>,

    pub included_instances: Option<HashSet<LxdInstanceName>>,
    pub excluded_instances: Option<HashSet<LxdInstanceName>>,

    pub included_statuses: Option<HashSet<LxdInstanceStatus>>,
    pub excluded_statuses: Option<HashSet<LxdInstanceStatus>>,

    pub keep_hourly: Option<usize>,
    pub keep_daily: Option<usize>,
    pub keep_weekly: Option<usize>,
    pub keep_monthly: Option<usize>,
    pub keep_yearly: Option<usize>,
    pub keep_last: Option<usize>,
    pub keep_limit: Option<usize>,
}

impl Policy {
    pub fn keep_hourly(&self) -> usize {
        self.keep_hourly.unwrap_or(0)
    }

    pub fn keep_daily(&self) -> usize {
        self.keep_daily.unwrap_or(0)
    }

    pub fn keep_weekly(&self) -> usize {
        self.keep_weekly.unwrap_or(0)
    }

    pub fn keep_monthly(&self) -> usize {
        self.keep_monthly.unwrap_or(0)
    }

    pub fn keep_yearly(&self) -> usize {
        self.keep_yearly.unwrap_or(0)
    }

    pub fn keep_last(&self) -> usize {
        self.keep_last.unwrap_or(0)
    }

    pub fn keep_limit(&self) -> Option<usize> {
        self.keep_limit
    }

    /// Returns whether this policy applies to given project and instance.
    ///
    /// For instance: if this policy is restricted to match only instances with
    /// specified names, it will return `true` only for instance matching
    /// those names.
    pub fn applies_to(&self, project: &LxdProject, instance: &LxdInstance) -> bool {
        fn in_set<T: Hash + Eq>(items: &Option<HashSet<T>>, item: &T, default: bool) -> bool {
            items
                .as_ref()
                .map(|items| items.contains(item))
                .unwrap_or(default)
        }

        let project_excluded = in_set(&self.excluded_projects, &project.name, false);
        let name_excluded = in_set(&self.excluded_instances, &instance.name, false);
        let status_excluded = in_set(&self.excluded_statuses, &instance.status, false);

        if project_excluded || name_excluded || status_excluded {
            return false;
        }

        let project_included = in_set(&self.included_projects, &project.name, true);
        let name_included = in_set(&self.included_instances, &instance.name, true);
        let status_included = in_set(&self.included_statuses, &instance.status, true);

        project_included && name_included && status_included
    }

    /// Merges this policy with another one, overwriting fields in a cascading
    /// fashion.
    pub fn merge_with(self, other: Self) -> Self {
        Self {
            included_projects: None,
            excluded_projects: None,
            included_instances: None,
            excluded_instances: None,
            included_statuses: None,
            excluded_statuses: None,
            keep_hourly: other.keep_hourly.or(self.keep_hourly),
            keep_daily: other.keep_daily.or(self.keep_daily),
            keep_weekly: other.keep_weekly.or(self.keep_weekly),
            keep_monthly: other.keep_monthly.or(self.keep_monthly),
            keep_yearly: other.keep_yearly.or(self.keep_yearly),
            keep_last: other.keep_last.or(self.keep_last),
            keep_limit: other.keep_limit.or(self.keep_limit),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_lxd::test_utils::*;
    use pretty_assertions as pa;

    fn project_a() -> LxdProject {
        project("a")
    }

    fn project_b() -> LxdProject {
        project("b")
    }

    fn projects(names: &[&str]) -> HashSet<LxdProjectName> {
        names.iter().map(LxdProjectName::new).collect()
    }

    fn instance_a() -> LxdInstance {
        LxdInstance {
            name: instance_name("a"),
            status: LxdInstanceStatus::Running,
            snapshots: Default::default(),
        }
    }

    fn instance_b() -> LxdInstance {
        LxdInstance {
            name: instance_name("b"),
            status: LxdInstanceStatus::Aborting,
            snapshots: Default::default(),
        }
    }

    fn instance_c() -> LxdInstance {
        LxdInstance {
            name: instance_name("c"),
            status: LxdInstanceStatus::Stopped,
            snapshots: Default::default(),
        }
    }

    fn instances(names: &[&str]) -> HashSet<LxdInstanceName> {
        names.iter().map(LxdInstanceName::new).collect()
    }

    fn statuses(statuses: &[LxdInstanceStatus]) -> HashSet<LxdInstanceStatus> {
        statuses.iter().cloned().collect()
    }

    mod applies_to {
        use super::*;

        mod policy_with_no_restrictions {
            use super::*;

            #[test]
            fn applies_to_every_instance() {
                let policy = Policy::default();

                assert_eq!(policy.applies_to(&project_a(), &instance_a()), true);
                assert_eq!(policy.applies_to(&project_a(), &instance_b()), true);
                assert_eq!(policy.applies_to(&project_b(), &instance_c()), true);
            }
        }

        mod policy_with_included_projects {
            use super::*;

            #[test]
            fn applies_only_to_instances_belonging_to_given_project() {
                let policy = Policy {
                    included_projects: Some(projects(&["a", "d"])),
                    ..Default::default()
                };

                assert_eq!(policy.applies_to(&project_a(), &instance_a()), true);
                assert_eq!(policy.applies_to(&project_a(), &instance_b()), true);
                assert_eq!(policy.applies_to(&project_b(), &instance_c()), false);
            }
        }

        mod policy_with_excluded_projects {
            use super::*;

            #[test]
            fn applies_to_all_instances_except_the_ones_belonging_to_given_project() {
                let policy = Policy {
                    excluded_projects: Some(projects(&["a", "d"])),
                    ..Default::default()
                };

                assert_eq!(policy.applies_to(&project_a(), &instance_a()), false);
                assert_eq!(policy.applies_to(&project_a(), &instance_b()), false);
                assert_eq!(policy.applies_to(&project_b(), &instance_c()), true);
            }
        }

        mod policy_with_included_instances {
            use super::*;

            #[test]
            fn applies_only_to_instances_with_given_names() {
                let policy = Policy {
                    included_instances: Some(instances(&["a", "d"])),
                    ..Default::default()
                };

                assert_eq!(policy.applies_to(&project_a(), &instance_a()), true);
                assert_eq!(policy.applies_to(&project_a(), &instance_b()), false);
                assert_eq!(policy.applies_to(&project_b(), &instance_c()), false);
            }
        }

        mod policy_with_excluded_instances {
            use super::*;

            #[test]
            fn applies_to_all_instances_except_the_ones_with_given_names() {
                let policy = Policy {
                    excluded_instances: Some(instances(&["a", "d"])),
                    ..Default::default()
                };

                assert_eq!(policy.applies_to(&project_a(), &instance_a()), false);
                assert_eq!(policy.applies_to(&project_a(), &instance_b()), true);
                assert_eq!(policy.applies_to(&project_b(), &instance_c()), true);
            }
        }

        mod policy_with_included_statuses {
            use super::*;

            #[test]
            fn applies_only_to_instances_with_given_statuses() {
                let policy = Policy {
                    included_statuses: Some(statuses(&[
                        LxdInstanceStatus::Aborting,
                        LxdInstanceStatus::Stopped,
                    ])),
                    ..Default::default()
                };

                assert_eq!(policy.applies_to(&project_a(), &instance_a()), false);
                assert_eq!(policy.applies_to(&project_a(), &instance_b()), true);
                assert_eq!(policy.applies_to(&project_b(), &instance_c()), true);
            }
        }

        mod policy_with_excluded_statuses {
            use super::*;

            #[test]
            fn applies_to_all_instances_except_the_ones_with_given_statuses() {
                let policy = Policy {
                    excluded_statuses: Some(statuses(&[
                        LxdInstanceStatus::Aborting,
                        LxdInstanceStatus::Stopped,
                    ])),
                    ..Default::default()
                };

                assert_eq!(policy.applies_to(&project_a(), &instance_a()), true);
                assert_eq!(policy.applies_to(&project_a(), &instance_b()), false);
                assert_eq!(policy.applies_to(&project_b(), &instance_c()), false);
            }
        }

        mod policy_with_included_projects_and_included_instances_and_included_statuses {
            use super::*;

            #[test]
            fn applies_only_to_instances_with_given_projects_and_names_and_statuses() {
                let policy = Policy {
                    included_projects: Some(projects(&["b", "c"])),
                    included_instances: Some(instances(&["c"])),
                    included_statuses: Some(statuses(&[
                        LxdInstanceStatus::Aborting,
                        LxdInstanceStatus::Stopped,
                    ])),
                    ..Default::default()
                };

                assert_eq!(policy.applies_to(&project_a(), &instance_a()), false);
                assert_eq!(policy.applies_to(&project_a(), &instance_b()), false);
                assert_eq!(policy.applies_to(&project_b(), &instance_c()), true);
            }
        }

        mod policy_with_excluded_projects_and_excluded_instances_and_excluded_statuses {
            use super::*;

            #[test]
            fn applies_to_all_instances_except_the_ones_with_given_projects_and_names_and_statuses()
            {
                let policy = Policy {
                    excluded_projects: Some(projects(&["b", "c"])),
                    excluded_instances: Some(instances(&["c"])),
                    excluded_statuses: Some(statuses(&[
                        LxdInstanceStatus::Aborting,
                        LxdInstanceStatus::Stopped,
                    ])),
                    ..Default::default()
                };

                assert_eq!(policy.applies_to(&project_a(), &instance_a()), true);
                assert_eq!(policy.applies_to(&project_a(), &instance_b()), false);
                assert_eq!(policy.applies_to(&project_b(), &instance_c()), false);
            }
        }
    }

    mod merge_with {
        use super::*;

        #[test]
        fn test() {
            // Policy A: has all values set, serves as a base
            let policy_a = Policy {
                keep_daily: Some(10),
                keep_weekly: Some(5),
                keep_monthly: Some(2),
                keep_yearly: Some(1),
                keep_last: Some(8),
                ..Default::default()
            };

            // Policy B: overwrites only the `keep weekly` and `keep monthly` options
            let policy_b = Policy {
                keep_weekly: Some(100),
                keep_monthly: Some(200),
                ..Default::default()
            };

            // Policy C: overwrites only the `keep yearly` option
            let policy_c = Policy {
                keep_yearly: Some(100),
                ..Default::default()
            };

            // Policy A + B
            let policy_ab = Policy {
                keep_weekly: Some(100),  // Overwritten from policy B
                keep_monthly: Some(200), // Overwritten from policy B
                ..policy_a.clone()
            };

            // Policy A + C
            let policy_ac = Policy {
                keep_yearly: Some(100), // Overwritten from policy C
                ..policy_a.clone()
            };

            pa::assert_eq!(policy_a.clone().merge_with(policy_b), policy_ab);
            pa::assert_eq!(policy_a.clone().merge_with(policy_c), policy_ac);
        }
    }
}
