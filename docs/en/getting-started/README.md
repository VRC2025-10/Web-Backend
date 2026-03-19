# Getting Started

> **Navigation**: [Docs Home](../README.md) > Getting Started

Welcome to the **VRC Backend** — a Rust/Axum REST API powering the VRC web platform. This guide will help you set up a local development environment, run the server, and start making API calls in minutes.

## Who Is This For?

- **Backend developers** contributing to the VRC platform
- **Frontend developers** integrating with the VRC API
- **System administrators** deploying the VRC backend in production
- **Community members** exploring the project

## What Is VRC Backend?

VRC Backend is a high-performance REST API built with Rust and Axum, backed by PostgreSQL. It provides endpoints for managing members, events, clubs, galleries, and more — with Discord OAuth2 authentication and role-based authorization.

## Guides

| Guide | Description |
|-------|-------------|
| [Installation](installation.md) | Full installation guide with prerequisites, Docker setup, manual setup, and production deployment |
| [Quickstart](quickstart.md) | Get up and running in 5 minutes |
| [Examples](examples.md) | Curated API usage examples with curl commands |

## Quick Links

- **Health check**: `GET /health`
- **API base**: `/api/v1/{public,internal,system,auth}`
- **Default port**: `3000`

## Related Documents

- [API Documentation](../../api/README.md)
- [Architecture Overview](../../../specs/02-architecture/README.md)
- [Security](../../../specs/06-security/README.md)
