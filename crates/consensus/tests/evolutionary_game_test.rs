//! Evolutionary Game Theory Integration Tests — Sprint 140
//!
//! Demuestra que la Simbiosis es un Equilibrio de Nash Evolutivamente Estable (ESS),
//! sin usar incentivos financieros (tokens/crypto).

use ed2k_consensus::{EvolutionaryGameEngine, ReplicatorConfig};

/// Nodo Altruista Simbiótico: alta coherencia, bajo costo, limpio
fn altruistic_params() -> (f64, f64, f64, f64) {
    (1.0, 0.1, 1.0, 0.0) // (tcm_coherence, energy_cost, diversity_entropy, byzantine_score)
}

/// Nodo Parásito Egoísta: baja coherencia, alto costo, sin diversidad
fn parasitic_params() -> (f64, f64, f64, f64) {
    (0.1, 0.9, 0.0, 1.0)
}

/// Nodo Bizantino: coherencia media, alto costo, malicioso
fn byzantine_params() -> (f64, f64, f64, f64) {
    (0.3, 0.7, 0.0, 2.0)
}

/// Nodo Neutral: en equilibrio con bar_f
fn neutral_params() -> (f64, f64, f64, f64) {
    (0.8, 0.3, 0.5, 0.1)
}

#[test]
fn test_nash_equilibrium_altruist_wins() {
    let engine = EvolutionaryGameEngine::default();

    let (coh, cost, div, biz) = altruistic_params();
    let results = engine.simulate(0.25, coh, cost, div, biz, 200);
    let final_share = results.last().unwrap().new_share;

    println!(
        "Altruista: share {:.4} -> {:.4}",
        0.25, final_share
    );
    assert!(
        final_share > 0.95,
        "Altruista debe dominar la red (share -> 1), got: {:.4}",
        final_share
    );
}

#[test]
fn test_nash_equilibrium_parasite_eliminated() {
    let engine = EvolutionaryGameEngine::default();

    let (coh, cost, div, biz) = parasitic_params();
    let results = engine.simulate(0.25, coh, cost, div, biz, 200);
    let final_share = results.last().unwrap().new_share;

    println!(
        "Parásito: share {:.4} -> {:.4}",
        0.25, final_share
    );
    assert!(
        final_share < 0.01,
        "Parásito debe ser eliminado (share -> 0), got: {:.4}",
        final_share
    );
}

#[test]
fn test_nash_equilibrium_byzantine_eliminated() {
    let engine = EvolutionaryGameEngine::default();

    let (coh, cost, div, biz) = byzantine_params();
    let results = engine.simulate(0.25, coh, cost, div, biz, 200);
    let final_share = results.last().unwrap().new_share;

    println!(
        "Bizantino: share {:.4} -> {:.4}",
        0.25, final_share
    );
    assert!(
        final_share < 0.001,
        "Bizantino debe ser eliminado rápidamente, got: {:.6}",
        final_share
    );
}

#[test]
fn test_nash_equilibrium_neutral_stable() {
    let engine = EvolutionaryGameEngine::default();

    let (coh, cost, div, biz) = neutral_params();
    let results = engine.simulate(0.25, coh, cost, div, biz, 200);
    let final_share = results.last().unwrap().new_share;

    println!(
        "Neutral: share {:.4} -> {:.4}",
        0.25, final_share
    );
    // En equilibrio: fitness_i - bar_f + η*C - δ*B ≈ 0
    // share debe permanecer estable
    assert!(
        (final_share - 0.25).abs() < 0.05,
        "Neutral debe permanecer estable cerca del share inicial, got: {:.4}",
        final_share
    );
}

#[test]
fn test_full_nash_equilibrium_demonstration() {
    // Demostración completa del Equilibrio de Nash Evolutivamente Estable (ESS)
    let engine = EvolutionaryGameEngine::default();
    let initial_share = 0.25; // Cada estrategia empieza con 25% de influencia

    println!("=== Nash Equilibrium Demonstration (ESS) ===");
    println!("Initial share per strategy: {:.2}", initial_share);
    println!();

    // Estrategia Simbiótica (Altruista)
    let (coh, cost, div, biz) = altruistic_params();
    let symbiotic = engine.simulate(initial_share, coh, cost, div, biz, 200);
    let symbiotic_final = symbiotic.last().unwrap().new_share;

    // Estrategia Parásita (Egoísta)
    let (coh, cost, div, biz) = parasitic_params();
    let parasitic = engine.simulate(initial_share, coh, cost, div, biz, 200);
    let parasitic_final = parasitic.last().unwrap().new_share;

    // Estrategia Bizantina (Maliciosa)
    let (coh, cost, div, biz) = byzantine_params();
    let byzantine = engine.simulate(initial_share, coh, cost, div, biz, 200);
    let byzantine_final = byzantine.last().unwrap().new_share;

    // Estrategia Neutral (Equilibrio)
    let (coh, cost, div, biz) = neutral_params();
    let neutral = engine.simulate(initial_share, coh, cost, div, biz, 200);
    let neutral_final = neutral.last().unwrap().new_share;

    println!("Results after 200 steps:");
    println!("  Simbiótica (Altruista): {:.4}", symbiotic_final);
    println!("  Parásita (Egoísta):     {:.4}", parasitic_final);
    println!("  Bizantina (Maliciosa):  {:.4}", byzantine_final);
    println!("  Neutral (Equilibrio):   {:.4}", neutral_final);
    println!();

    // Total influence remaining
    let total = symbiotic_final + parasitic_final + byzantine_final + neutral_final;
    println!("Total influence: {:.4}", total);
    println!();

    // ESS Properties:
    // 1. La estrategia simbiótica domina
    assert!(
        symbiotic_final > 0.95,
        "ESS-1: Simbiótica debe dominar (share -> 1)"
    );

    // 2. Las estrategias no-cooperativas son eliminadas
    assert!(
        parasitic_final < 0.01,
        "ESS-2: Parásita debe ser eliminada"
    );
    assert!(
        byzantine_final < 0.001,
        "ESS-3: Bizantina debe ser eliminada"
    );

    // 3. La estrategia neutral permanece estable
    assert!(
        (neutral_final - initial_share).abs() < 0.05,
        "ESS-4: Neutral debe permanecer estable"
    );

    // 4. Simbiótica > todas las demás
    assert!(symbiotic_final > parasitic_final, "ESS-5: Simbiótica > Parásita");
    assert!(symbiotic_final > byzantine_final, "ESS-6: Simbiótica > Bizantina");
    assert!(symbiotic_final > neutral_final, "ESS-7: Simbiótica > Neutral");

    println!("✅ Nash Equilibrium Verified: Simbiosis is an Evolutionarily Stable Strategy (ESS)");
    println!("   No financial incentives required — pure thermodynamic survival!");
}

#[test]
fn test_tragedy_of_commons_resolved() {
    // Demostración de que la Tragedia de los Comunes se resuelve sin dinero
    let engine = EvolutionaryGameEngine::default();

    // Escenario: Red con 4 tipos de nodos, todos empezando igual
    let strategies = vec![
        ("Altruista", altruistic_params()),
        ("Parásito", parasitic_params()),
        ("Bizantino", byzantine_params()),
        ("Neutral", neutral_params()),
    ];

    let mut shares: Vec<(String, f64)> = strategies
        .iter()
        .map(|(name, _)| (name.to_string(), 0.25))
        .collect();

    println!("=== Tragedia de los Comunes -> Resuelta ===");
    println!("Inicio: Todos con 25% de influencia");
    println!();

    // Simular 100 pasos
    for step in 0..100 {
        for (i, (_name, params)) in strategies.iter().enumerate() {
            let (coh, cost, div, biz) = *params;
            let result = engine.compute_replicator_dynamics(
                shares[i].1,
                coh,
                cost,
                div,
                biz,
            );
            shares[i].1 = result.new_share;
        }

        // Imprimir progreso cada 20 pasos
        if step % 20 == 0 || step == 99 {
            println!("Paso {}: ", step);
            for (name, share) in &shares {
                println!("  {:12}: {:.4}", name, share);
            }
            println!();
        }
    }

    // Verificar resultados finales
    let altruist_share = shares[0].1;
    let parasite_share = shares[1].1;
    let byzantine_share = shares[2].1;

    println!("Conclusion:");
    println!(
        "  Altruista domina con {:.1}% de influencia",
        altruist_share * 100.0
    );
    println!(
        "  Parásito reducido a {:.2}% de influencia",
        parasite_share * 100.0
    );
    println!(
        "  Bizantino eliminado a {:.4}% de influencia",
        byzantine_share * 100.0
    );

    assert!(altruist_share > 0.9, "Altruista debe dominar");
    assert!(parasite_share < 0.05, "Parásito debe ser eliminado");
    assert!(byzantine_share < 0.001, "Bizantino debe ser eliminado");

    println!();
    println!(
        "✅ Tragedia de los Comunes Resuelta: La cooperacion es la estrategia optima!"
    );
}

#[test]
fn test_replicator_config_import() {
    // Verifica que ReplicatorConfig es exportable desde el crate
    let cfg = ReplicatorConfig::default()
        .with_bar_f(0.6)
        .with_eta(0.15)
        .with_delta(0.75);
    assert!((cfg.bar_f - 0.6).abs() < 1e-9);
    assert!((cfg.eta - 0.15).abs() < 1e-9);
    assert!((cfg.delta - 0.75).abs() < 1e-9);

    let engine = EvolutionaryGameEngine::new(cfg);
    assert!((engine.config.bar_f - 0.6).abs() < 1e-9);
}

#[test]
fn test_energy_efficiency_proof() {
    // Demostrar que el nodo más eficiente energéticamente gana
    let engine = EvolutionaryGameEngine::default();

    // Nodo eficiente: misma coherencia, menor costo
    let efficient = engine.simulate(0.5, 0.8, 0.05, 0.5, 0.0, 100);
    let efficient_final = efficient.last().unwrap().new_share;

    // Nodo ineficiente: misma coherencia, mayor costo
    let inefficient = engine.simulate(0.5, 0.8, 0.5, 0.5, 0.0, 100);
    let inefficient_final = inefficient.last().unwrap().new_share;

    println!("Eficiente:     {:.4}", efficient_final);
    println!("Ineficiente:   {:.4}", inefficient_final);

    assert!(
        efficient_final > inefficient_final,
        "Nodo eficiente debe ganar a nodo ineficiente"
    );
    assert!(efficient_final > 0.5, "Eficiente debe ganar influencia");
    assert!(
        inefficient_final < 0.5,
        "Ineficiente debe perder influencia"
    );
}
