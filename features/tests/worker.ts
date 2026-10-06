import path from "node:path";

/// Each Playwright worker gets its own server, database, and demo library, so
/// workers run side by side. Playwright numbers the workers 0 to workers - 1.
export const WORKER = Number(process.env.TEST_PARALLEL_INDEX ?? "0");

export const PORT = 3100 + WORKER;
export const BASE_URL = `http://127.0.0.1:${PORT}`;

/// test.sh creates one database per worker, named after DATABASE_URL.
export const DATABASE_URL = `${process.env.DATABASE_URL}_${WORKER}`;

export const LIBRARY = path.resolve(__dirname, `../.library-${WORKER}`);
