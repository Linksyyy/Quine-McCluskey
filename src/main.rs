use comfy_table::Table;
use std::fs;
use std::io;
use std::process;

#[derive(Debug, Clone)]
struct Minterm {
    name: String,
    inputs: String,
    bit_size: usize,
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
    println!("Truth table:\n{truth_table}\n");

    // (minterm name, inputs, number of bits 1)
    let mut list: Vec<Minterm> = Vec::new();

    println!("Organizing data:");
    for (i, line) in truth_table.lines().enumerate() {
        if line.ends_with("1")
            && input_amount > 0
            && let Some(input_bits) = line.get(..input_amount)
        {
            let minterm_name = format!("m({})", i);

            list.push(Minterm {
                name: minterm_name,
                inputs: input_bits.to_string(),
                bit_size: input_bits.matches('1').count(),
            });
            if let Some(last) = list.last() {
                println!("{} -> {} -> {}", last.name, last.inputs, last.bit_size);
            }
        }
    }
    println!("\nSpliting into groups with 1 bit diff:");

    // (minterm name, input)
    let mut groups: Vec<Vec<Minterm>> = Vec::new();
    for term in list.iter() {
        if term.bit_size >= groups.len() {
            groups.resize(term.bit_size + 1, Vec::new());
        }
        groups[term.bit_size].push(term.clone());
    }

    for (i, group) in groups.iter().enumerate() {
        if !group.is_empty() {
            println!("group {i}: {:?}", group.iter().cloned());
        }
    }
    println!("\nComparing groups:");
    let mut compared: Vec<(String, String, Minterm, Minterm)> = Vec::new();
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
                    for (c_char, l_char) in term_a.inputs.chars().zip(term_b.inputs.chars()) {
                        if c_char == l_char {
                            result.push(l_char);
                        } else {
                            result.push('-');
                        }
                    }
                    if !result.chars().all(|c| c == result.chars().next().unwrap()) {
                        acc += 1;
                        compared.push((format!("T{acc}"), result, term_a.clone(), term_b.clone()));
                    }
                }
            }
        }
    }

    let mut table = Table::new();
    table.set_header(vec!["Combined term", "Binary", "Coverage"]);
    for term in compared.iter() {
        table.add_row(vec![
            term.0.clone(),
            term.1.clone(),
            format!("{},{}", term.2.name, term.3.name),
        ]);
    }
    println!("{table}");

    println!("\nSecond comparing flow:");

    let mut second_compared: Vec<(String, String)> = Vec::new();

    let mut acc = 0;
    for (i, current) in compared.iter().enumerate() {
        for next in compared.iter().skip(i + 1) {
            let mut result = String::new();
            for (c_char, l_char) in current.1.chars().zip(next.1.chars()) {
                if c_char == l_char {
                    result.push(l_char);
                } else {
                    result.push('-');
                }
            }
            println!(
                "{} {} & {} {} -> {}",
                current.1, current.0, next.1, next.0, result
            );
            if !result.chars().all(|c| c == result.chars().next().unwrap()) {
                acc += 1;
                second_compared.push((format!("S{acc}"), result));
            }
        }
    }

    let mut table = Table::new();
    table.set_header(vec!["Combined term", "binary", "Boolean exp", "Coverage"]);
    for term in second_compared {
        println!("{:?}", term);
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
