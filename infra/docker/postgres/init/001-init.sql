CREATE DATABASE meat_memory_dev;
CREATE DATABASE meat_memory_test;

\connect meat_memory_dev
CREATE EXTENSION IF NOT EXISTS vector;

\connect meat_memory_test
CREATE EXTENSION IF NOT EXISTS vector;
