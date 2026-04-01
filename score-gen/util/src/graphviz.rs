use frontend::archetypes::RunnableArchetypeDescription;
use std::collections::HashMap;

/// Generate a Graphviz DOT representation of the runnable dataflow graph.
///
/// Each entry in `archetypes` is a `(namespace, runnable)` pair.
/// Edges are inferred by matching output `type_name` to input `type_name`.
pub fn generate_dot(
    system_name: &str,
    archetypes: &[(String, &RunnableArchetypeDescription)],
) -> String {
    let mut dot = String::new();
    dot.push_str(&format!("digraph \"{}\" {{\n", system_name));
    dot.push_str("    rankdir=LR;\n");
    dot.push_str("    node [shape=record, style=filled];\n");
    dot.push_str("    edge [fontsize=10];\n\n");

    // Build a map: output type_name -> list of (namespace, runnable, output_port_name)
    let mut producers: HashMap<&str, Vec<(&str, &str, &str)>> = HashMap::new();
    // Build a map: input type_name -> list of (namespace, runnable, input_port_name)
    let mut consumers: HashMap<&str, Vec<(&str, &str, &str)>> = HashMap::new();

    for (ns, r) in archetypes {
        for out in &r.outputs {
            producers
                .entry(&out.type_name)
                .or_default()
                .push((ns, &r.unique_name, &out.unique_name));
        }
        for inp in &r.inputs {
            consumers
                .entry(&inp.type_name)
                .or_default()
                .push((ns, &r.unique_name, &inp.unique_name));
        }
    }

    // Emit nodes
    for (ns, r) in archetypes {
        let is_source = r.inputs.is_empty() && !r.outputs.is_empty();
        let is_sink = !r.inputs.is_empty() && r.outputs.is_empty();

        let color = if is_source {
            "#a8d5ba" // green — source
        } else if is_sink {
            "#f5a6a6" // red — sink
        } else {
            "#a6c8f5" // blue — intermediate
        };

        let node_id = format!("{}_{}", ns, r.unique_name);

        let mut input_ports = String::new();
        for inp in &r.inputs {
            let short_type = inp.type_name.rsplit("::").next().unwrap_or(&inp.type_name);
            input_ports.push_str(&format!(
                "<{}> {}\\n({})|",
                inp.unique_name, inp.unique_name, short_type
            ));
        }

        let mut output_ports = String::new();
        for out in &r.outputs {
            let short_type = out.type_name.rsplit("::").next().unwrap_or(&out.type_name);
            output_ports.push_str(&format!(
                "<{}> {}\\n({})|",
                out.unique_name, out.unique_name, short_type
            ));
        }

        let role = if is_source {
            "SOURCE"
        } else if is_sink {
            "SINK"
        } else {
            "PROCESS"
        };

        let label = if input_ports.is_empty() && output_ports.is_empty() {
            format!("{}\\n[{}]\\n{}", r.unique_name, r.runnabletype, role)
        } else {
            let inputs_section = if input_ports.is_empty() {
                String::new()
            } else {
                format!("{{{}}}", input_ports.trim_end_matches('|'))
            };
            let outputs_section = if output_ports.is_empty() {
                String::new()
            } else {
                format!("{{{}}}", output_ports.trim_end_matches('|'))
            };

            let mut sections = Vec::new();
            if !inputs_section.is_empty() {
                sections.push(inputs_section);
            }
            sections.push(format!(
                "{}\\n[{}]\\n{}",
                r.unique_name, r.runnabletype, role
            ));
            if !outputs_section.is_empty() {
                sections.push(outputs_section);
            }
            sections.join("|")
        };

        dot.push_str(&format!(
            "    {} [label=\"{}\", fillcolor=\"{}\"];\n",
            node_id, label, color
        ));
    }

    dot.push('\n');

    // Emit edges: connect producers to consumers with matching type_name
    for (type_name, prods) in &producers {
        if let Some(cons) = consumers.get(type_name) {
            let short_type = type_name.rsplit("::").next().unwrap_or(type_name);
            for (p_ns, p_name, p_port) in prods {
                for (c_ns, c_name, c_port) in cons {
                    dot.push_str(&format!(
                        "    {}_{}: {} -> {}_{}: {} [label=\"{}\"];\n",
                        p_ns, p_name, p_port, c_ns, c_name, c_port, short_type
                    ));
                }
            }
        }
    }

    dot.push_str("}\n");
    dot
}

#[cfg(test)]
mod tests {
    use super::*;
    use frontend::archetypes::{
        BuildInformation, RunnableArchetypeDescription, RunnableInputDescription,
        RunnableOutputDescription,
    };

    fn make_runnable(
        name: &str,
        rtype: &str,
        inputs: Vec<(&str, &str)>,
        outputs: Vec<(&str, &str)>,
    ) -> RunnableArchetypeDescription {
        RunnableArchetypeDescription {
            unique_name: name.to_string(),
            runnabletype: rtype.to_string(),
            inputs: inputs
                .into_iter()
                .map(|(n, t)| RunnableInputDescription {
                    unique_name: n.to_string(),
                    type_name: t.to_string(),
                    ..Default::default()
                })
                .collect(),
            outputs: outputs
                .into_iter()
                .map(|(n, t)| RunnableOutputDescription {
                    unique_name: n.to_string(),
                    type_name: t.to_string(),
                    n_samples_max: 1,
                    ..Default::default()
                })
                .collect(),
            build_information: BuildInformation {
                bazel_target: String::new(),
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_source_node_is_green() {
        let r = make_runnable("Sensor", "CPU", vec![], vec![("out", "Data")]);
        let dot = generate_dot("test", &[("NS".to_string(), &r)]);
        assert!(dot.contains("#a8d5ba")); // green
        assert!(dot.contains("SOURCE"));
    }

    #[test]
    fn test_sink_node_is_red() {
        let r = make_runnable("Logger", "CPU", vec![("in", "Data")], vec![]);
        let dot = generate_dot("test", &[("NS".to_string(), &r)]);
        assert!(dot.contains("#f5a6a6")); // red
        assert!(dot.contains("SINK"));
    }

    #[test]
    fn test_edge_by_type_name() {
        let producer = make_runnable("Cam", "CPU", vec![], vec![("frames", "img::Frame")]);
        let consumer = make_runnable("Det", "GPU", vec![("input", "img::Frame")], vec![]);
        let archetypes = vec![("NS".to_string(), &producer), ("NS".to_string(), &consumer)];
        let dot = generate_dot("test", &archetypes);
        assert!(dot.contains("NS_Cam: frames -> NS_Det: input"));
        assert!(dot.contains("Frame")); // short type on edge label
    }

    #[test]
    fn test_no_edge_when_types_differ() {
        let producer = make_runnable("A", "CPU", vec![], vec![("out", "TypeA")]);
        let consumer = make_runnable("B", "CPU", vec![("in", "TypeB")], vec![]);
        let archetypes = vec![("NS".to_string(), &producer), ("NS".to_string(), &consumer)];
        let dot = generate_dot("test", &archetypes);
        assert!(!dot.contains("->"));
    }

    #[test]
    fn test_digraph_structure() {
        let r = make_runnable("X", "CPU", vec![], vec![]);
        let dot = generate_dot("my-system", &[("NS".to_string(), &r)]);
        assert!(dot.starts_with("digraph \"my-system\""));
        assert!(dot.ends_with("}\n"));
        assert!(dot.contains("rankdir=LR"));
    }
}
