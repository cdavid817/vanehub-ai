import { createServer } from "node:net";

const loopbackHost = "127.0.0.1";

function listen(server, port) {
  return new Promise((resolve, reject) => {
    const onError = (error) => {
      server.off("listening", onListening);
      reject(error);
    };
    const onListening = () => {
      server.off("error", onError);
      resolve();
    };
    server.once("error", onError);
    server.once("listening", onListening);
    server.listen(port, loopbackHost);
  });
}

function close(server) {
  return new Promise((resolve, reject) => {
    server.close((error) => {
      if (error) reject(error);
      else resolve();
    });
  });
}

export function parseRequestedPort(value) {
  if (value === undefined || value === "") return 0;
  if (!/^\d+$/.test(value)) {
    throw new Error(`DOCS_SCREENSHOT_PORT must be an integer, received "${value}".`);
  }
  const port = Number(value);
  if (!Number.isSafeInteger(port) || port < 1 || port > 65_535) {
    throw new Error(`DOCS_SCREENSHOT_PORT must be between 1 and 65535, received "${value}".`);
  }
  return port;
}

export async function allocateScreenshotPort(requestedValue) {
  const requestedPort = parseRequestedPort(requestedValue);
  const server = createServer();
  server.unref();

  try {
    await listen(server, requestedPort);
    const address = server.address();
    if (!address || typeof address === "string") {
      throw new Error("Unable to resolve the allocated documentation screenshot port.");
    }
    return address.port;
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    const label = requestedPort === 0 ? "an available loopback port" : `loopback port ${requestedPort}`;
    throw new Error(`Unable to reserve ${label} for documentation screenshots: ${detail}`);
  } finally {
    if (server.listening) await close(server);
  }
}
