use std::{fs, path::PathBuf, process::Command};

use ::git2 as git;

use ::gitkit::{
    cli::{cli, Cli, Remove},
    Result,
};

fn main() {
    let cli = cli();
    split(cli).unwrap();
}

const BRANCH_PREFIX: &str = "temp_split_";

fn split(cli: &Cli) -> Result<()> {
    println!("{:#?}", cli);

    let repo_pb = &PathBuf::from(&cli.repo);
    let path_pb = &repo_pb.join(&cli.path);
    let target_pb = &PathBuf::from(&cli.target);

    assert!(repo_pb.is_dir() && repo_pb.exists());
    assert!(path_pb.is_dir() && path_pb.exists());

    if !target_pb.exists() {
        fs::create_dir_all(target_pb)?;
    } else {
        assert!(target_pb.is_dir());
    }

    let repo_path = &cli.repo;
    let path_path = &cli.path;
    let target_path = &cli.target;

    // fit for windows
    let repo_git = &repo_path.to_string().replace("\\", "/");
    let path_git = &path_path.to_string().replace("\\", "/");

    let repo = git::Repository::open(repo_path)?;

    for status in repo.statuses(None)?.into_iter() {
        if status.status() != git::Status::CURRENT {
            return Err(git::Error::new(
                git::ErrorCode::Uncommitted,
                git::ErrorClass::Filter,
                "you have unstaged changes",
            )
            .into());
        }
    }

    match git::Repository::open(target_path) {
        Ok(_) => (),
        Err(e) if e.code() == git::ErrorCode::NotFound => {
            git::Repository::init(target_path)?;
        }
        Err(e) => {
            return Err(e.into());
        }
    };

    let temp_branch_name = &{
        let mut temp_branch_name = BRANCH_PREFIX.to_string();
        while repo
            .find_branch(&temp_branch_name, git::BranchType::Local)
            .is_ok()
        {
            temp_branch_name.push('_');
        }
        temp_branch_name
    };

    println!("waiting for subtree to finish ...");

    Command::new("git")
        .current_dir(repo_path)
        .arg("subtree")
        .arg("split")
        .arg("-P")
        .arg(path_git)
        .arg("-b")
        .arg(temp_branch_name)
        .spawn()?
        .wait()?;

    let mut branch = repo.find_branch(temp_branch_name, git::BranchType::Local)?;

    println!("subtree finished.");

    println!(
        "remote adding '{}' to '{}' ...",
        temp_branch_name, target_path
    );

    Command::new("git")
        .current_dir(target_path)
        .arg("pull")
        .arg(repo_git)
        .arg(temp_branch_name)
        .spawn()?
        .wait()?;

    println!("remote added.");

    branch.delete()?;

    match cli.remove {
        Remove::Nothing => println!("remove nothing"),
        Remove::Commit => {
            println!("rm -rf {:?} ...", path_pb);
            std::fs::remove_dir_all(path_pb)?;
            let mut index = repo.index()?;
            index.add_all(["*"].iter(), git::IndexAddOption::DEFAULT, None)?;
            index.write()?;
            println!("done.");
        }
        Remove::Prune => {
            println!("pruning ...");

            Command::new("git")
                .current_dir(repo_path)
                .arg("filter-branch")
                .arg("--index-filter")
                .arg("git")
                .arg("rm -rf --cached --ignore-unmatch")
                .arg(path_git)
                .arg("--prune-empty")
                .arg("--")
                .arg("--all")
                .spawn()?
                .wait()?;

            let for_each_ref = Command::new("git")
                .current_dir(repo_path)
                .arg("for-each-ref")
                .arg("--format=%(refname)")
                .arg("refs/original/")
                .output()?;
            for refname in String::from_utf8_lossy(&for_each_ref.stdout).lines() {
                Command::new("git")
                    .current_dir(repo_path)
                    .arg("update-ref")
                    .arg("-d")
                    .arg(refname)
                    .spawn()?
                    .wait()?;
            }

            Command::new("git")
                .current_dir(repo_path)
                .arg("reflog")
                .arg("expire")
                .arg("--expire=now")
                .arg("--all")
                .spawn()?
                .wait()?;

            Command::new("git")
                .current_dir(repo_path)
                .arg("gc")
                .arg("--aggressive")
                .arg("--prune=now")
                .spawn()?
                .wait()?;
        }
    };

    Ok(())
}
