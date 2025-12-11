# Rust Axum CRUD API + SQLxData

> This code is a modification of [FrancescoXX/axum-crud-api](https://github.com/FrancescoXX/axum-crud-api) adapted to use SQLx-Data.

A high-performance CRUD REST API built with **Rust**, **Axum**, **SQLx**, and **Postgres**.

## Architecture

The application consists of a Rust backend (Axum) communicating with a Postgres database, orchestrated via Docker Compose.

## 🚀 Tech Stack

- **Language:** Rust 🦀
- **Framework:** [Axum](https://github.com/tokio-rs/axum) (0.8)
- **Database:** PostgreSQL
- **ORM/Querying:** SQLx + SQLxData
- **Containerization:** Docker & Docker Compose
- **Rest Client:** VsCode Rest Client extension to run request.http


## 🛠️ Prerequisites

Before starting, ensure you have the following installed:
- [Docker Desktop](https://www.docker.com/products/docker-desktop/)
- [Rust & Cargo](https://www.rust-lang.org/tools/install) (optional if running fully in Docker)

## 🏁 Getting Started

You can run the entire application (App + Database) using a single command.

DATABASE_URL="postgres://user:password@localhost:5432/simple_api" cargo run