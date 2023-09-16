use clap::Parser;
use oort_simulator::simulation::Code;
use oort_simulator::{scenario, simulation};
use rayon::prelude::*;
use std::default::Default;

#[derive(Parser, Debug)]
#[clap()]
struct Arguments {
    scenario: String,
    shortcodes: Vec<String>,

    #[clap(short, long, default_value = "10")]
    rounds: u32,

    #[clap(short, long)]
    dev: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("battle=info"))
        .init();

    let args = Arguments::parse();
    scenario::load_safe(&args.scenario).expect("Unknown scenario");
    if args.shortcodes.len() < 2 {
        panic!("Expected at least two shortcodes");
    }

    log::info!("Compiling AIs");
    let http = reqwest::Client::new();
    let ais = oort_tools::fetch_and_compile_multiple(&http, &args.shortcodes, args.dev).await?;

    log::info!("Running simulations");
    let player0 = &ais[0];
    let results_per_opponent = ais[1..]
        .par_iter()
        .map(|player1| {
            let codes = vec![player0.compiled_code.clone(), player1.compiled_code.clone()];
            let results = run_simulations(&args.scenario, codes, args.rounds);
            (player1, results)
        })
        .collect::<Vec<_>>();

    for (player1, results) in results_per_opponent {
        let n = 10;
        println!("{} vs {}:", player0.name, player1.name);
        println!(
            "  Wins: {} {:?}",
            results.team0_wins.len(),
            &results.team0_wins[..].iter().take(n).collect::<Vec<_>>()
        );
        println!(
            "  Losses: {} {:?}",
            results.team1_wins.len(),
            &results.team1_wins[..].iter().take(n).collect::<Vec<_>>()
        );
        println!(
            "  Draws: {} {:?}",
            results.draws.len(),
            &results.draws[..].iter().take(n).collect::<Vec<_>>()
        );
    }

    Ok(())
}

#[derive(Default, Debug)]
struct Results {
    team0_wins: Vec<u32>,
    team1_wins: Vec<u32>,
    draws: Vec<u32>,
}

fn run_simulations(scenario_name: &str, codes: Vec<Code>, rounds: u32) -> Results {
    let seed_statuses: Vec<(u32, scenario::Status)> = (0..rounds)
        .into_par_iter()
        .map(|seed| (seed, run_simulation(scenario_name, seed, codes.clone())))
        .collect();
    let mut results: Results = Default::default();
    for (seed, status) in seed_statuses {
        match status {
            scenario::Status::Victory { team: 0 } => results.team0_wins.push(seed),
            scenario::Status::Victory { team: 1 } => results.team1_wins.push(seed),
            scenario::Status::Draw => results.draws.push(seed),
            _ => unreachable!(),
        }
    }
    results
}

fn run_simulation(scenario_name: &str, seed: u32, codes: Vec<Code>) -> scenario::Status {
    let mut sim = simulation::Simulation::new(scenario_name, seed, &codes);
    while sim.status() == scenario::Status::Running && sim.tick() < scenario::MAX_TICKS {
        sim.step();
    }
    sim.status()
}
