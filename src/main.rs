use comfy_table::Table;
use std::fs;
use std::io;
use std::process;

#[derive(Debug, Clone)]
struct Term {
    name: String,
    binary: String,
    bit_size: usize,
    coverage: Vec<String>,
}

fn main() {
    let selected_path = select_path("benchmark");
    let selected_file = &select_path(&selected_path);

    let content = fs::read_to_string(selected_file).expect("FUDEU");
    let config = content
        .lines()
        .filter(|line| line.starts_with("."))
        .collect::<Vec<_>>()
        .join("\n");
    let truth_table = content
        .lines()
        .filter(|line| !line.starts_with("."))
        .map(|line| line.split("#").next().unwrap_or("").trim())
        .collect::<Vec<_>>()
        .join("\n");

    let mut input_amount: usize = 0;
    let mut _output_amount: usize = 0;

    println!("\nReading configs from ({selected_file}) ...");
    for line in config.lines() {
        let line = match line.find("#") {
            #[allow(clippy::needless_borrow)]
            Some(val) => line[..val].trim(),
            None => line.trim(),
        };

        if line.starts_with(".i") {
            input_amount = line
                .split_whitespace()
                .nth(1)
                .unwrap()
                .parse::<usize>()
                .unwrap();
            println!("inputs: {input_amount}");
        }
        if line.starts_with(".o") {
            _output_amount = line.chars().last().unwrap_or('0').to_digit(10).unwrap() as usize;
            println!("outputs: {_output_amount}");
        }
        if line.starts_with(".e") {
            println!("Done!\n");
        }
    }

    let mut table = Table::new();
    table.set_header(vec!["input", "output"]);
    for line in truth_table.lines() {
        let line = line.split_whitespace();
        table.add_row(vec![
            line.clone().next().unwrap_or("").to_string(),
            line.clone().nth(1).unwrap_or("").to_string(),
        ]);
    }
    println!("Truth table:\n{table}\n");

    // (minterm name, inputs, number of bits 1)
    let mut list: Vec<Term> = Vec::new();

    println!("Organizing data:");
    for (i, line) in truth_table.lines().enumerate() {
        if line.ends_with("1")
            && input_amount > 0
            && let Some(input_bits) = line.get(..input_amount)
        {
            let minterm_name = format!("m({})", i);

            list.push(Term {
                name: minterm_name.clone(),
                binary: input_bits.to_string(),
                bit_size: input_bits.matches('1').count(),
                coverage: vec![minterm_name],
            });
            if let Some(last) = list.last() {
                println!("{} -> {} -> {}", last.name, last.binary, last.bit_size);
            }
        }
    }
    println!("\nSpliting into groups with 1 bit diff:");

    let mut compared = factor_terms(&list, None);
    if compared.is_empty() {
        compared = list.clone();
    }

    let mut table = Table::new();
    table.set_header(vec!["Combined term", "Binary", "Coverage"]);
    for term in compared.iter() {
        table.add_row(vec![
            term.name.clone(),
            term.binary.clone(),
            term.coverage.join(","),
        ]);
    }
    println!("\n{table}");

    let minterms = list
        .clone()
        .iter()
        .map(|term| term.name.clone())
        .collect::<Vec<_>>();
    let coverages = compared
        .iter()
        .map(|term| {
            let mut coverage = term
                .coverage
                .iter()
                .filter(|name| name.starts_with("m("))
                .cloned()
                .collect::<Vec<_>>();
            coverage.sort();
            coverage.dedup();
            coverage
        })
        .collect::<Vec<_>>();

    let mut remaining = minterms.clone();
    let mut selected_indices: Vec<usize> = Vec::new();

    loop {
        let mut added = false;
        let current_remaining = remaining.clone();
        for minterm in current_remaining {
            let mut covering = Vec::new();
            for (idx, coverage) in coverages.iter().enumerate() {
                if coverage.contains(&minterm) {
                    covering.push(idx);
                }
            }
            if covering.len() == 1 {
                let idx = covering[0];
                if !selected_indices.contains(&idx) {
                    selected_indices.push(idx);
                    added = true;
                }
            }
        }

        if !added {
            break;
        }

        remaining.retain(|minterm| {
            !selected_indices
                .iter()
                .any(|idx| coverages[*idx].contains(minterm))
        });

        if remaining.is_empty() {
            break;
        }
    }

    if !remaining.is_empty() {
        let mut clauses: Vec<Vec<usize>> = Vec::new();
        for minterm in &remaining {
            let mut coverers = Vec::new();
            for (idx, coverage) in coverages.iter().enumerate() {
                if selected_indices.contains(&idx) {
                    continue;
                }
                if coverage.contains(minterm) {
                    coverers.push(idx);
                }
            }
            if coverers.is_empty() {
                println!("ERROR: minterm {minterm} is not covered by any implicant");
                process::exit(0);
            }
            clauses.push(coverers);
        }

        let products = petrick(&clauses);
        let implicant_costs = compared
            .iter()
            .map(|term| literal_count(&term.binary))
            .collect::<Vec<_>>();
        if let Some(best) = choose_best_product(products, &implicant_costs) {
            for idx in best {
                if !selected_indices.contains(&idx) {
                    selected_indices.push(idx);
                }
            }
        }
    }

    let selected_terms = selected_indices
        .into_iter()
        .map(|idx| compared[idx].clone())
        .collect::<Vec<_>>();

    let variables = (0..input_amount)
        .map(|idx| format!("x{}", idx + 1))
        .collect::<Vec<_>>();

    let mut expression = String::new();
    for (i, term) in selected_terms.iter().enumerate() {
        for (i, char) in term.binary.chars().enumerate() {
            match char {
                '-' => continue,
                '0' => expression.push_str(&format!("~{}", variables[i])),
                '1' => expression.push_str(&variables[i].to_string()),
                _ => (),
            }
        }
        if i != selected_terms.len().saturating_sub(1) {
            expression.push_str(" + ");
        }
    }
    println!("Final expression:\n{expression}");
}

fn literal_count(binary: &str) -> usize {
    binary.chars().filter(|char| *char != '-').count()
}

fn is_subset(smaller: &[usize], larger: &[usize]) -> bool {
    if smaller.len() > larger.len() {
        return false;
    }
    let mut idx = 0;
    for value in larger {
        if idx < smaller.len() && smaller[idx] == *value {
            idx += 1;
        }
        if idx == smaller.len() {
            return true;
        }
    }
    idx == smaller.len()
}

fn minimize_products(mut products: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
    for product in products.iter_mut() {
        product.sort_unstable();
        product.dedup();
    }
    products.sort();
    products.dedup();
    products.sort_by(|a, b| a.len().cmp(&b.len()).then(a.cmp(b)));

    let mut minimized: Vec<Vec<usize>> = Vec::new();
    'outer: for product in products {
        for existing in &minimized {
            if is_subset(existing, &product) {
                continue 'outer;
            }
        }
        minimized.push(product);
    }
    minimized
}

fn petrick(clauses: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut products: Vec<Vec<usize>> = vec![Vec::new()];
    for clause in clauses {
        let mut next: Vec<Vec<usize>> = Vec::new();
        for product in &products {
            for &implicant in clause {
                let mut new_product = product.clone();
                if !new_product.contains(&implicant) {
                    new_product.push(implicant);
                }
                next.push(new_product);
            }
        }
        products = minimize_products(next);
    }
    products
}

fn choose_best_product(
    products: Vec<Vec<usize>>,
    implicant_costs: &[usize],
) -> Option<Vec<usize>> {
    let mut best_product: Option<Vec<usize>> = None;
    let mut best_cost = (usize::MAX, usize::MAX);
    for product in products {
        let literal_total = product.iter().map(|idx| implicant_costs[*idx]).sum();
        let cost = (product.len(), literal_total);
        let is_better = match &best_product {
            Some(current) => {
                cost < best_cost || (cost == best_cost && product.as_slice() < current.as_slice())
            }
            None => true,
        };
        if is_better {
            best_cost = cost;
            best_product = Some(product);
        }
    }
    best_product
}

fn factor_terms(list: &[Term], recursive_number: Option<u32>) -> Vec<Term> {
    let recursive_iterator = recursive_number.unwrap_or(1);
    let mut groups: Vec<Vec<Term>> = Vec::new();
    for term in list.iter() {
        if term.bit_size >= groups.len() {
            groups.resize(term.bit_size + 1, Vec::new());
        }
        groups[term.bit_size].push(term.clone());
    }

    let has_groups = groups.iter().any(|group| !group.is_empty());
    if has_groups {
        let mut table = Table::new();
        table.set_header(vec!["Group", "Terms"]);
        for (i, group) in groups.iter().enumerate() {
            if group.is_empty() {
                continue;
            }
            let terms = group
                .iter()
                .map(|el| el.binary.clone())
                .collect::<Vec<_>>()
                .join(", ");
            table.add_row(vec![i.to_string(), terms]);
        }
        println!("\nIteration {recursive_iterator}:\n{table}");
    }
    let mut compared: Vec<Term> = Vec::new();
    let mut combined_any = false;
    for i in 0..groups.len() {
        if i >= groups.len() {
            break;
        }

        if let Some(current) = &groups.get(i)
            && let Some(next) = &groups.get(i + 1)
        {
            let mut acc = 0;
            for term_a in current.iter() {
                for term_b in next.iter() {
                    let mut result = String::new();
                    let mut diff = 0;
                    let mut compatible = true;
                    for (c_char, l_char) in term_a.binary.chars().zip(term_b.binary.chars()) {
                        if c_char == l_char {
                            result.push(l_char);
                        } else if c_char == '-' || l_char == '-' {
                            compatible = false;
                            break;
                        } else {
                            result.push('-');
                            diff += 1;
                        }
                    }
                    if compatible && diff == 1 {
                        acc += 1;
                        combined_any = true;
                        let mut coverage = term_a.coverage.clone();
                        coverage.extend(term_b.coverage.iter().cloned());
                        compared.push(Term {
                            name: format!("T{acc}"),
                            bit_size: result.matches('1').count(),
                            binary: result,
                            coverage,
                        });
                    }
                }
            }
        }
    }

    if combined_any {
        factor_terms(&compared, Some(recursive_iterator + 1))
    } else {
        list.to_vec()
    }
}

fn select_path(path: &str) -> String {
    let dirs = fs::read_dir(path).unwrap();
    let mut dirs_str = String::new();
    for dir in dirs {
        dirs_str.push_str(&format!("{}\n", dir.unwrap().path().to_str().unwrap()));
    }
    let total_lines = dirs_str.lines().count();
    println!("Choose dir of tests:");
    for (i, path) in dirs_str.lines().enumerate() {
        println!("{} - {path}", i + 1);
    }
    println!("Type the number of your choice: ");
    let mut input_choice = String::new();
    io::stdin().read_line(&mut input_choice).unwrap();
    let sanitized_input = match input_choice.trim().parse::<i32>() {
        Ok(num) if (1..total_lines + 1).contains(&(num as usize)) => num - 1,
        Ok(num) => {
            println!("ERROR: INPUT ({num}) OUT OF RANGE (1..{})", total_lines);
            process::exit(0);
        }
        Err(_) => {
            println!("ERROR: INVALID INPUT");
            process::exit(0)
        }
    };
    dirs_str
        .lines()
        .nth(sanitized_input as usize)
        .unwrap_or("ERROR")
        .to_string()
}
