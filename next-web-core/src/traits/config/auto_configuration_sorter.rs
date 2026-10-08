//! Ordering of the configurations of an application.
//!
//! The configurations of an application are applied in the order of
//! [`Ordered::order`](crate::Ordered::order), except for the pairs of
//! configurations that declare a place relative to each other with
//! [`ConfigurationOrder::configuration_before`] and
//! [`ConfigurationOrder::configuration_after`]. The sorting is a topological
//! sort, so that a configuration is applied before every configuration its
//! names ask for, and the order and the name of a configuration decide the
//! order of the configurations that are not constrained at the same time.
//!
//! The same sorter orders the [`AutoConfiguration`]s of a starter and the
//! [`Configuration`]s of an application, because the two declare the same place
//! with the same rules.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use tracing::trace;

use crate::Ordered;
use crate::traits::config::auto_configuration::AutoConfiguration;
use crate::traits::config::configuration::Configuration;

/// The metadata the sorter of the configurations of an application reads.
///
/// Both the [`AutoConfiguration`]s of a starter and the [`Configuration`]s of
/// an application are sorted by the same rules, so the sorter is generic over
/// the two and reads their metadata through this trait.
pub trait ConfigurationOrder {
    /// Returns the name the `before` and `after` lists refer to the
    /// configuration with.
    fn configuration_name(&self) -> &'static str;

    /// Returns the names of the configurations this one is applied before.
    fn configuration_before(&self) -> &'static [&'static str];

    /// Returns the names of the configurations this one is applied after.
    fn configuration_after(&self) -> &'static [&'static str];

    /// Returns the order the configuration is applied in.
    fn configuration_order(&self) -> i32;
}

impl ConfigurationOrder for Box<dyn AutoConfiguration> {
    fn configuration_name(&self) -> &'static str {
        AutoConfiguration::auto_configuration_name(&**self)
    }

    fn configuration_before(&self) -> &'static [&'static str] {
        AutoConfiguration::auto_configure_before(&**self)
    }

    fn configuration_after(&self) -> &'static [&'static str] {
        AutoConfiguration::auto_configure_after(&**self)
    }

    fn configuration_order(&self) -> i32 {
        Ordered::order(&**self)
    }
}

impl ConfigurationOrder for Box<dyn Configuration> {
    fn configuration_name(&self) -> &'static str {
        Configuration::configuration_name(&**self)
    }

    fn configuration_before(&self) -> &'static [&'static str] {
        Configuration::configure_before(&**self)
    }

    fn configuration_after(&self) -> &'static [&'static str] {
        Configuration::configure_after(&**self)
    }

    fn configuration_order(&self) -> i32 {
        Ordered::order(&**self)
    }
}

/// Orders the auto-configurations of an application.
///
/// This is the counterpart of the auto-configuration sorter of Spring Boot: the
/// configurations of an application name the ones they have to be applied
/// before or after, which is what lets a configuration of an application take a
/// place among the configurations of the starters it depends on. A name no
/// configuration provides is ignored, so that a starter can name a
/// configuration an application may or may not use.
pub struct AutoConfigurationSorter;

/// Orders the configurations of an application.
///
/// The configurations of an application are ordered by the same rules the
/// auto-configurations of a starter are ordered by, so this is the same sorter
/// under the name the application side of the framework uses.
pub type ConfigurationSorter = AutoConfigurationSorter;

impl AutoConfigurationSorter {
    /// Returns the configurations in the order they are applied in.
    ///
    /// The configurations that do not constrain each other keep the order their
    /// [`order`](crate::Ordered::order) declares, and the configurations of the
    /// same order keep the order of their
    /// [`auto_configuration_name`](AutoConfiguration::auto_configuration_name),
    /// which keeps the result of the sorting the same for every run of an
    /// application.
    ///
    /// # Errors
    ///
    /// Returns an error when the names the configurations declare form a cycle,
    /// which is a cycle of configurations that each has to be applied before
    /// the next one.
    pub fn sort<T>(configurations: Vec<T>) -> Result<Vec<T>, AutoConfigurationOrderError>
    where
        T: ConfigurationOrder,
    {
        let order = Self::sort_indices(&configurations)?;

        let mut configurations: Vec<Option<T>> = configurations.into_iter().map(Some).collect();

        Ok(order
            .into_iter()
            .filter_map(|index| configurations[index].take())
            .collect())
    }

    /// Returns the positions of the configurations in the order they are applied
    /// in.
    ///
    /// # Errors
    ///
    /// Returns an error when the names the configurations declare form a cycle.
    ///
    /// # Arguments
    ///
    /// * `configurations` - The configurations to order.
    fn sort_indices<T>(configurations: &[T]) -> Result<Vec<usize>, AutoConfigurationOrderError>
    where
        T: ConfigurationOrder,
    {
        let count = configurations.len();

        // A name is the identity of a configuration, and a name more than one
        // configuration provides names every one of them.
        let mut by_name: HashMap<&'static str, Vec<usize>> = HashMap::new();
        for (index, configuration) in configurations.iter().enumerate() {
            by_name
                .entry(configuration.configuration_name())
                .or_default()
                .push(index);
        }

        // The configurations that have to be applied before a configuration.
        let mut prerequisites: Vec<Vec<usize>> = vec![Vec::new(); count];

        for (index, configuration) in configurations.iter().enumerate() {
            let name = configuration.configuration_name();

            for &target_name in configuration.configuration_before() {
                let Some(targets) = by_name.get(target_name) else {
                    trace!(
                        configuration = name,
                        before = target_name,
                        "no configuration provides this name, so the constraint is ignored"
                    );

                    continue;
                };

                for &target in targets {
                    // A configuration that names itself holds no constraint.
                    if target == index {
                        continue;
                    }

                    prerequisites[target].push(index);
                }
            }

            for &source_name in configuration.configuration_after() {
                let Some(sources) = by_name.get(source_name) else {
                    trace!(
                        configuration = name,
                        after = source_name,
                        "no configuration provides this name, so the constraint is ignored"
                    );

                    continue;
                };

                for &source in sources {
                    if source == index {
                        continue;
                    }

                    prerequisites[index].push(source);
                }
            }
        }

        // The order and the name of a configuration decide the order of the
        // configurations that do not constrain each other, and a constrained
        // configuration is applied next to the configuration it names.
        let mut in_order: Vec<usize> = (0..count).collect();
        in_order.sort_by_key(|index| {
            let configuration = &configurations[*index];

            (
                configuration.configuration_order(),
                configuration.configuration_name(),
                *index,
            )
        });

        Ok(Self::visit(&in_order, &prerequisites, configurations)?)
    }

    /// Returns the configurations in the order they are applied in, which is the
    /// order of [`Self::sort_indices`] without the configurations that
    /// constrain each other.
    ///
    /// The configurations are visited in the order their [`Ordered::order`]
    /// declares, and the configurations that have to be applied before a
    /// configuration are visited first, so that a configuration takes the place
    /// of the configuration it names and leaves the order of the configurations
    /// that do not constrain each other alone.
    ///
    /// # Errors
    ///
    /// Returns an error when the names the configurations declare form a cycle,
    /// which is a chain of configurations that each has to be applied before
    /// the next one.
    ///
    /// # Arguments
    ///
    /// * `in_order` - The positions of the configurations in the order they
    ///   declare.
    /// * `prerequisites` - The configurations that have to be applied before a
    ///   configuration.
    /// * `configurations` - The configurations to order.
    fn visit<T>(
        in_order: &[usize],
        prerequisites: &[Vec<usize>],
        configurations: &[T],
    ) -> Result<Vec<usize>, AutoConfigurationOrderError>
    where
        T: ConfigurationOrder,
    {
        /// Whether the configurations that have to be applied before a
        /// configuration were visited.
        #[derive(Clone, Copy, PartialEq)]
        enum Visit {
            /// The configuration was not visited yet.
            Unvisited,
            /// The configuration is on the chain that is being visited, so a
            /// prerequisite that leads back to it is a cycle.
            Visiting,
            /// The configuration was applied.
            Applied,
        }

        let mut visited = vec![Visit::Unvisited; configurations.len()];
        let mut applied = Vec::with_capacity(configurations.len());

        // The chain of configurations that is being visited, and the
        // prerequisite of each of them that is visited next.
        let mut chain: Vec<(usize, usize)> = Vec::new();

        for &start in in_order {
            if visited[start] == Visit::Applied {
                continue;
            }

            visited[start] = Visit::Visiting;
            chain.push((start, 0));

            while let Some((index, position)) = chain.pop() {
                let Some(&prerequisite) = prerequisites[index].get(position) else {
                    visited[index] = Visit::Applied;
                    applied.push(index);

                    continue;
                };

                chain.push((index, position + 1));

                match visited[prerequisite] {
                    Visit::Applied => {}
                    Visit::Visiting => {
                        return Err(AutoConfigurationOrderError::cycle(
                            chain
                                .iter()
                                .skip_while(|(index, _)| *index != prerequisite)
                                .map(|(index, _)| configurations[*index].configuration_name())
                                .chain([configurations[prerequisite].configuration_name()])
                                .collect(),
                        ));
                    }
                    Visit::Unvisited => {
                        visited[prerequisite] = Visit::Visiting;
                        chain.push((prerequisite, 0));
                    }
                }
            }
        }

        Ok(applied)
    }
}

/// The error reported when the auto-configurations of an application cannot be
/// ordered.
#[derive(Debug)]
pub struct AutoConfigurationOrderError {
    /// The names of the configurations of the cycle, which starts and ends with
    /// the configuration the cycle was entered at.
    configurations: Vec<String>,
}

impl AutoConfigurationOrderError {
    /// Returns the names of the configurations that form the cycle.
    pub fn configurations(&self) -> &[String] {
        &self.configurations
    }

    /// Returns the error of a cycle of configurations.
    ///
    /// # Arguments
    ///
    /// * `configurations` - The names of the configurations of the cycle, which
    ///   starts and ends with the configuration the cycle was entered at.
    fn cycle(configurations: Vec<&'static str>) -> Self {
        Self {
            configurations: configurations.into_iter().map(str::to_owned).collect(),
        }
    }
}

impl fmt::Display for AutoConfigurationOrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the auto-configurations cannot be ordered, because the names of their `before` and \
             `after` lists form a cycle: {}",
            self.configurations.join(" -> ")
        )
    }
}

impl Error for AutoConfigurationOrderError {}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use async_trait::async_trait;

    use super::*;
    use crate::{ApplicationContext, Ordered};

    /// A configuration the tests order, which declares its name and its
    /// constraints directly instead of deriving them from its type.
    #[derive(Clone)]
    struct Fake {
        name: &'static str,
        order: i32,
        before: &'static [&'static str],
        after: &'static [&'static str],
    }

    impl Fake {
        fn new(name: &'static str) -> Self {
            Self {
                name,
                order: 100,
                before: &[],
                after: &[],
            }
        }

        fn with_order(mut self, order: i32) -> Self {
            self.order = order;
            self
        }

        fn before(mut self, before: &'static [&'static str]) -> Self {
            self.before = before;
            self
        }

        fn after(mut self, after: &'static [&'static str]) -> Self {
            self.after = after;
            self
        }
    }

    #[async_trait]
    impl AutoConfiguration for Fake {
        async fn configure(
            &mut self,
            _ctx: &mut dyn ApplicationContext,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }

        fn auto_configuration_name(&self) -> &'static str {
            self.name
        }

        fn auto_configure_before(&self) -> &'static [&'static str] {
            self.before
        }

        fn auto_configure_after(&self) -> &'static [&'static str] {
            self.after
        }
    }

    impl Ordered for Fake {
        fn order(&self) -> i32 {
            self.order
        }
    }

    /// A configuration that takes its name from its type, which is what a
    /// configuration of a starter that does not declare a name does.
    #[derive(Clone)]
    struct NamedAfterItsType;

    #[async_trait]
    impl AutoConfiguration for NamedAfterItsType {
        async fn configure(
            &mut self,
            _ctx: &mut dyn ApplicationContext,
        ) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }

    impl Ordered for NamedAfterItsType {}

    /// Returns the configurations as the type erased ones the sorter orders.
    fn boxed(configurations: Vec<Fake>) -> Vec<Box<dyn AutoConfiguration>> {
        configurations
            .into_iter()
            .map(|configuration| Box::new(configuration) as Box<dyn AutoConfiguration>)
            .collect()
    }

    /// Returns the names of the configurations in the order they are applied in.
    fn order_of(configurations: Vec<Fake>) -> Vec<&'static str> {
        AutoConfigurationSorter::sort(boxed(configurations))
            .expect("the configurations are orderable")
            .iter()
            .map(|configuration| configuration.auto_configuration_name())
            .collect()
    }

    #[test]
    fn names_a_configuration_after_its_type() {
        let configuration: Box<dyn AutoConfiguration> = Box::new(NamedAfterItsType);

        assert_eq!(configuration.auto_configuration_name(), "NamedAfterItsType");
        assert!(configuration.auto_configure_before().is_empty());
        assert!(configuration.auto_configure_after().is_empty());
    }

    #[test]
    fn orders_configurations_that_declare_only_an_order() {
        assert_eq!(
            order_of(vec![
                Fake::new("late").with_order(200),
                Fake::new("early").with_order(10),
                Fake::new("middle"),
            ]),
            vec!["early", "middle", "late"]
        );
    }

    #[test]
    fn orders_configurations_of_the_same_order_by_name() {
        assert_eq!(
            order_of(vec![
                Fake::new("charlie"),
                Fake::new("alpha"),
                Fake::new("bravo"),
            ]),
            vec!["alpha", "bravo", "charlie"]
        );
    }

    #[test]
    fn applies_a_configuration_before_the_one_it_names() {
        assert_eq!(
            order_of(vec![
                Fake::new("second").with_order(10),
                Fake::new("first").with_order(200).before(&["second"]),
            ]),
            vec!["first", "second"]
        );
    }

    #[test]
    fn applies_a_configuration_after_the_one_it_names() {
        assert_eq!(
            order_of(vec![
                Fake::new("first").with_order(200),
                Fake::new("second").with_order(10).after(&["first"]),
            ]),
            vec!["first", "second"]
        );
    }

    #[test]
    fn ignores_a_name_no_configuration_provides() {
        assert_eq!(
            order_of(vec![
                Fake::new("only").before(&["missing"]).after(&["missing"]),
            ]),
            vec!["only"]
        );
    }

    #[test]
    fn ignores_a_configuration_that_names_itself() {
        assert_eq!(
            order_of(vec![Fake::new("only").before(&["only"]).after(&["only"]),]),
            vec!["only"]
        );
    }

    #[test]
    fn applies_a_name_to_every_configuration_that_provides_it() {
        assert_eq!(
            order_of(vec![
                Fake::new("first").before(&["second"]),
                Fake::new("second"),
                Fake::new("second"),
            ]),
            vec!["first", "second", "second"]
        );
    }

    #[test]
    fn applies_the_transitive_constraints_of_the_configurations() {
        assert_eq!(
            order_of(vec![
                Fake::new("first").before(&["second"]),
                Fake::new("second").before(&["third"]),
                Fake::new("third").with_order(10),
            ]),
            vec!["first", "second", "third"]
        );
    }

    #[test]
    fn keeps_the_order_of_the_configurations_that_do_not_constrain_each_other() {
        // The constrained configuration declares the highest order, so it is
        // applied before `early` instead of being applied after the
        // configurations that only declare an order.
        assert_eq!(
            order_of(vec![
                Fake::new("early").with_order(10),
                Fake::new("middle").with_order(20),
                Fake::new("last").with_order(100),
                Fake::new("constrained").with_order(200).before(&["early"]),
            ]),
            vec!["constrained", "early", "middle", "last"]
        );
    }

    #[test]
    fn reports_the_configurations_that_form_a_cycle() {
        let error = AutoConfigurationSorter::sort(boxed(vec![
            Fake::new("first").before(&["second"]),
            Fake::new("second").before(&["first"]),
            Fake::new("third").after(&["first"]),
        ]))
        .err()
        .expect("a cycle cannot be ordered");

        // The cycle is reported as the chain of names that leads back to the
        // configuration the chain was entered at, and not the configurations
        // that only wait for the cycle.
        assert_eq!(
            error.configurations().join(" -> "),
            "first -> second -> first"
        );
        assert!(error.to_string().contains("first -> second -> first"));
    }
}
