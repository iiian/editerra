use crate::error::MapEdiError;
use crate::expr_engine::*;
use crate::map_doc::*;

use self::Composite;
use self::ConditionalLoop;
use self::ConditionalSegment;
use self::NormalLoop;
use self::NormalSegment;

pub fn map_edi(
    map: EdiMapDoc,
    delimiters: &Delimiters,
    engine: &mut ExprEngine,
) -> Result<Option<String>, MapEdiError> {
    /*
     * `map_edi` is essentially a tree-walking interpreter.
     * It is broken up into two large steps:
     *  1. Build an intermediate representation by walking the EDI map
     *  2. Convert that intermediate representation into EDI.
     *
     * The two steps correspond to the two function calls, `evaluate_scope` and
     * `ir2edi` below.
     *
     * The purpose for an intermediate representation is to make the suppression
     * system more maintainable. Suppression essentially necessitates the
     * existence of *some* intermediate representation, because the EDI
     * suppression system is fundamentally hierarchical.
     */
    let scope = map.map;
    let chunks = map.chunks;
    let tree = evaluate_scope(engine, &scope, &chunks)?;
    ir2edi(&tree, delimiters)
}

struct LoopNode {
    name: String,
    components: Option<Vec<Box<IRNode>>>,
    required: bool,
}

struct SegmentNode {
    name: String,
    elements: Option<Vec<IRElement>>,
    required: bool,
}

// -- implementation ----------------------------------------------------------------------------------
enum IRNode {
    LoopNode(LoopNode),
    SegmentNode(SegmentNode),
}

struct IRElementComposite {
    index: u8,
    val: Vec<IRElementSimple>,
    required: bool,
}

enum IRElement {
    Normal(IRElementSimple),
    Composite(IRElementComposite),
}
impl IRElement {
    fn composite_mut(&mut self) -> Option<&mut IRElementComposite> {
        match self {
            IRElement::Normal(_) => None,
            IRElement::Composite(c) => Some(c),
        }
    }

    fn normal_mut(&mut self) -> Option<&mut IRElementSimple> {
        match self {
            IRElement::Normal(n) => Some(n),
            IRElement::Composite(_) => None,
        }
    }

    fn composite(&self) -> Option<&IRElementComposite> {
        match self {
            IRElement::Normal(_) => None,
            IRElement::Composite(c) => Some(c),
        }
    }
}

pub struct IRElementSimple {
    index: u8,
    val: Option<String>,
    required: bool,
}

impl IRElementSimple {
    pub fn get_checked_value<F>(&self, f: F) -> Result<Option<String>, MapEdiError>
    where
        F: Fn() -> String,
    {
        match (self.required, &self.val) {
            (true, None) => Err(MapEdiError::UnmetRequirement(f())),
            (true, Some(x)) if x.len() == 0 => Err(MapEdiError::UnmetRequirement(f())),
            (_, Some(x)) => Ok(Some(x.clone())),
            (_, None) => Ok(None),
        }
    }
}

fn evaluate_scope(
    engine: &mut ExprEngine,
    scope: &Scope,
    chunks: &Option<Vec<Scope>>,
) -> Result<IRNode, MapEdiError> {
    match scope {
        Scope::Attach(Attach {
            attach,
            cnd,
            array,
            required,
            name,
        }) => {
            // Example:
            // ```
            // - (R)some_scope:
            //     if: <some expression>
            //     attach: /body/2300
            // ```
            if engine.eval_bool(cnd)? {
                let target_scope = chunks
                    .as_ref()
                    .expect("chunks must be populated")
                    .iter()
                    .find(|&e| {
                        if let Scope::Loop(loop_scope) = e {
                            match loop_scope {
                                Loop::Normal(NormalLoop { name, .. }) => name == attach,
                                Loop::Conditional(ConditionalLoop { name, .. }) => name == attach,
                            }
                        } else {
                            false
                        }
                    });
                if let Some(scope) = target_scope {
                    if *array {
                        Ok(IRNode::LoopNode(LoopNode {
                            name: format!("virt"),
                        }))
                    }
                    evaluate_scope(engine, scope, chunks)
                } else {
                    Ok(IRNode::LoopNode(LoopNode {
                        name: name.clone(),
                        components: None,
                        required: *required,
                    }))
                }
            } else {
                Ok(IRNode::LoopNode(LoopNode {
                    name: name.clone(),
                    components: None,
                    required: *required,
                }))
            }
        }
        Scope::Loop(loop_scope) => {
            // Example:
            // ```
            // - (R)some_scope:
            //     components:
            //     ...
            // ```
            match loop_scope {
                Loop::Normal(NormalLoop {
                    required,
                    context,
                    array,
                    components,
                    name,
                }) => evaluate_loop(components, context, engine, chunks, *required, name),
                Loop::Conditional(ConditionalLoop {
                    required,
                    name,
                    cnd,
                    array,
                    context,
                    then_components,
                    else_components,
                }) => {
                    if engine.eval_bool(cnd)? {
                        if let Some(components) = then_components {
                            evaluate_loop(components, context, engine, chunks, *required, name)
                        } else {
                            Ok(IRNode::SegmentNode(SegmentNode {
                                name: name.clone(),
                                elements: None,
                                required: *required,
                            }))
                        }
                    } else if let Some(components) = else_components {
                        evaluate_loop(components, context, engine, chunks, *required, name)
                    } else {
                        Ok(IRNode::SegmentNode(SegmentNode {
                            name: name.clone(),
                            elements: None,
                            required: *required,
                        }))
                    }
                }
            }
        }
        Scope::Segment(segment) => match segment {
            // Example:
            // ```
            // - (R)some_scope:
            //     elements:
            //     ...
            // ```
            Segment::Normal(NormalSegment {
                required,
                name,
                elements,
                after,
            }) => evaluate_segment(elements, engine, after, name, *required),
            Segment::Conditional(ConditionalSegment {
                required,
                name,
                cnd,
                then_elements,
                else_elements,
                after,
            }) => {
                if engine.eval_bool(cnd)? {
                    if let Some(then_elements) = then_elements {
                        evaluate_segment(then_elements, engine, after, name, *required)
                    } else {
                        Ok(IRNode::SegmentNode(SegmentNode {
                            name: name.clone(),
                            elements: None,
                            required: *required,
                        }))
                    }
                } else if let Some(else_elements) = else_elements {
                    evaluate_segment(else_elements, engine, after, name, *required)
                } else {
                    Ok(IRNode::SegmentNode(SegmentNode {
                        name: name.clone(),
                        elements: None,
                        required: *required,
                    }))
                }
            }
        },
    }
}

fn evaluate_loop(
    components: &Vec<Box<Scope>>,
    context: &Option<String>,
    engine: &mut ExprEngine,
    chunks: &Option<Vec<Scope>>,
    required: bool,
    name: &String,
) -> Result<IRNode, MapEdiError> {
    if let Some(context_expr) = context {
        engine.exec(&String::from("context_stack.push($)"))?;
        engine.exec(&format!("$ = {};", context_expr))?;
    }

    let constructed = extract_oks(
        components
            .iter()
            .map(|scope| evaluate_scope(engine, scope, chunks))
            .collect(),
    )?;

    if let Some(_) = context {
        engine.exec(&String::from("context_stack.pop();"))?;
        engine.exec(&String::from(
            "$ = context_stack[context_stack.length - 1];",
        ))?;
    }

    Ok(IRNode::LoopNode(LoopNode {
        name: name.clone(),
        components: Some(constructed.into_iter().map(|c| Box::new(c)).collect()),
        required,
    }))
}

fn evaluate_segment(
    elements: &Vec<Element>,
    engine: &mut ExprEngine,
    after: &Option<Vec<AfterRule>>,
    name: &String,
    required: bool,
) -> Result<IRNode, MapEdiError> {
    let mut constructed = extract_oks(
        elements
            .into_iter()
            .map(|e| evaluate_element(engine, e))
            .collect(),
    )?;
    if let Some(after_rules) = after {
        for rule in after_rules {
            handle_after_rule(rule, engine, name, &mut constructed)?;
        }
    }
    Ok(IRNode::SegmentNode(SegmentNode {
        name: name.clone(),
        elements: Some(constructed),
        required,
    }))
}

#[inline]
fn handle_after_rule(
    rule: &AfterRule,
    engine: &mut ExprEngine,
    name: &String,
    constructed: &mut Vec<IRElement>,
) -> Result<(), MapEdiError> {
    Ok(match rule {
        AfterRule::IfThenSuppress { cnd, target } => {
            handle_if_then_suppress(engine, cnd, target, name, constructed)?;
        }
        AfterRule::SuppressSegmentIf(cnd) => {
            if engine.eval_bool(cnd)? {
                *constructed = Vec::new();
            }
        }
    })
}

#[inline]
fn handle_if_then_suppress(
    engine: &mut ExprEngine,
    cnd: &String,
    target: &String,
    name: &String,
    constructed: &mut Vec<IRElement>,
) -> Result<(), MapEdiError> {
    Ok(if engine.eval_bool(cnd)? {
        let replace = &target.replace(name, "");
        let mut element_indices = replace.split("-");
        let major_index = element_indices
            .next()
            .unwrap()
            .parse::<u8>()
            .map_err(|_| MapEdiError::IfThenSuppress)?;
        if let Some(major_elem) = constructed.iter_mut().find(|x| match x.composite() {
            Some(x) => x.index == major_index,
            None => false,
        }) {
            if let Some(min_index_parse) = element_indices.next().map(|minor_index| {
                minor_index
                    .parse::<u8>()
                    .map_err(|_| MapEdiError::IfThenSuppress)
            }) {
                let minor_elem = min_index_parse?;
                let composite = major_elem.composite_mut().unwrap();
                if let Some(elem) = composite.val.iter_mut().find(|x| x.index == minor_elem) {
                    elem.val = None;
                }
            } else {
                major_elem.normal_mut().unwrap().val = None;
            }
        }
    })
}

// todo: OPTIMIZATION: this isn't strictly necessary, we could hand in a parametric type
fn extract_oks<T, E>(vec_of_results: Vec<Result<T, E>>) -> Result<Vec<T>, E> {
    vec_of_results.into_iter().collect()
}

fn evaluate_element(engine: &mut ExprEngine, element: &Element) -> Result<IRElement, MapEdiError> {
    match element {
        Element::Normal(BasicElement {
            required,
            index,
            mapping,
        }) => Ok(IRElement::Normal(IRElementSimple {
            index: *index,
            required: *required,
            val: engine.eval_string(mapping)?,
        })),
        Element::Composite(Composite {
            required,
            index,
            mappings,
        }) => {
            let mut constructed = Vec::new();
            for BasicElement {
                required,
                index,
                mapping,
            } in mappings
            {
                constructed.push(IRElementSimple {
                    index: *index,
                    val: engine.eval_string(mapping)?,
                    required: *required,
                })
            }

            Ok(IRElement::Composite(IRElementComposite {
                index: *index,
                val: constructed,
                required: *required,
            }))
        }
    }
}

fn ir2edi(node: &IRNode, del: &Delimiters) -> Result<Option<String>, MapEdiError> {
    let nl = del.nl.clone().unwrap_or_else(String::new);
    match node {
        IRNode::LoopNode(LoopNode {
            name,
            components,
            required,
        }) => {
            if let Some(components) = components {
                let subscopes = components
                    .into_iter()
                    .map(|c| ir2edi(c, del))
                    .collect::<Vec<_>>();

                let mut parts = Vec::new();
                for result in subscopes {
                    match result {
                        Ok(something) => parts.push(something),
                        Err(error) => {
                            if *required {
                                return Err(MapEdiError::UnmetRequirement(format!(
                                    "{}{}",
                                    name, error
                                )));
                            } else {
                                parts.push(None);
                            }
                        }
                    }
                }

                let mut result: String = String::new();
                for part in parts {
                    match part {
                        Some(part) => result.extend(part.chars()),
                        None => result.extend(nl.clone().chars()),
                    };
                }

                Ok(Some(result))
            } else if !required {
                Ok(None)
            } else {
                Err(MapEdiError::UnmetRequirement(name.clone()))
            }
        }
        IRNode::SegmentNode(SegmentNode {
            name,
            elements,
            required,
        }) => {
            let mut segment_elements: Vec<Option<String>> = Vec::new();
            match elements {
                Some(elements) => {
                    for element in elements {
                        match element {
                            IRElement::Normal(normal) => {
                                segment_elements.push(
                                    normal
                                        .get_checked_value(|| format!("{}{}", name, normal.index))?
                                        .map(|next| sanitize(next, del)),
                                );
                            }
                            IRElement::Composite(composite) => {
                                let mut subelements = Vec::new();
                                for subelement in &composite.val {
                                    subelements.push(
                                        subelement
                                            .get_checked_value(|| {
                                                format!(
                                                    "{}{}{}",
                                                    name, composite.index, subelement.index
                                                )
                                            })?
                                            .map(|next| sanitize(next, del)),
                                    );
                                }
                                let composite_element = subelements
                                    .into_iter()
                                    .map(|optelm| {
                                        optelm
                                            .and_then(|next| Some(sanitize(next, del)))
                                            .unwrap_or_else(String::new)
                                    })
                                    .collect::<Vec<_>>()
                                    .join(&del.co)
                                    .trim_end_matches(&del.co)
                                    .to_string();
                                if composite_element.len() == 0 && composite.required {
                                    return Err(MapEdiError::UnmetRequirement(
                                        format!("{}{}", name, composite.index).to_string(),
                                    ));
                                }
                                segment_elements.push(Some(composite_element));
                            }
                        }
                    }
                }
                None => {
                    if *required {
                        return Err(MapEdiError::UnmetRequirement(name.clone()));
                    }
                }
            }

            let start = std::iter::once(name.clone());
            let mut segment = start
                .chain(
                    segment_elements
                        .into_iter()
                        .map(|optelm| optelm.unwrap_or_else(String::new)),
                )
                .collect::<Vec<_>>()
                .join(&del.el)
                .trim_end_matches(&del.el)
                .to_string();
            segment.extend(del.le.chars());
            Ok(Some(segment))
        }
    }
}

fn sanitize(next: String, del: &Delimiters) -> String {
    if let Some(nl) = &del.nl {
        next.replace(nl, " ")
    } else {
        next
    }
    .replace(&del.co, " ")
    .replace(&del.el, " ")
    .replace(&del.le, " ")
}
