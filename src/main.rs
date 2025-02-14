use std::fs;
use std::process::Command;
#[macro_use]
extern crate clap;
use shiika;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = load_yaml!("cli.yml");
    let matches = clap::App::from(yaml).get_matches();

    if let Some(ref matches) = matches.subcommand_matches("compile") {
        let filepath = matches.value_of("INPUT").unwrap();
        compile(filepath)?;
    }

    if let Some(ref matches) = matches.subcommand_matches("run") {
        let filepath = matches.value_of("INPUT").unwrap();
        compile(filepath)?;
        run(filepath)?;
    }

    Ok(())
}

fn compile(filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
    let str = fs::read_to_string(filepath)?;
    let ast = shiika::parser::Parser::parse(&str)?;
    let stdlib = shiika::stdlib::Stdlib::create();
    let hir = shiika::hir::Hir::from_ast(ast, stdlib)?;
    let mut code_gen = shiika::code_gen::CodeGen::new();
    code_gen.gen_program(hir)?;
    code_gen.module.print_to_file(filepath.to_string() + ".ll")?;
    Ok(())
}

fn run(sk_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let ll_path = sk_path.to_string() + ".ll";
    let opt_ll_path = sk_path.to_string() + ".opt.ll";
    let bc_path = sk_path.to_string() + ".bc";
    let asm_path = sk_path.to_string() + ".s";
    let out_path = sk_path.to_string() + ".out";

    let mut cmd = Command::new("opt");
    cmd.arg("-O3");
    cmd.arg(ll_path);
    cmd.arg("-o");
    cmd.arg(bc_path.clone());
    cmd.output()?;

    let mut cmd = Command::new("llvm-dis");
    cmd.arg(bc_path.clone());
    cmd.arg("-o");
    cmd.arg(opt_ll_path);
    cmd.output()?;

    let mut cmd = Command::new("llc");
    cmd.arg(bc_path.clone());
    cmd.output()?;

    let mut cmd = Command::new("cc");
    cmd.arg("-I/usr/local/Cellar/bdw-gc/7.6.0/include/");
    cmd.arg("-L/usr/local/Cellar/bdw-gc/7.6.0/lib/");
    cmd.arg("-lgc");
    cmd.arg("-o");
    cmd.arg(out_path.clone());
    cmd.arg(asm_path.clone());
    cmd.output()?;

    fs::remove_file(bc_path)?;
    fs::remove_file(asm_path)?;

    let mut cmd = Command::new(out_path);
    cmd.status()?;

    Ok(())
}
