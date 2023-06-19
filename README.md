# Datadog Formatting Layer

This crate provides a layer for formatting events so Datadog can parse them

## Why not just .json()

Datadog expects a specific (mostly undocumented) json format.

This crate tries to mimic this format
