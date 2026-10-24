import type { Dispatch, SetStateAction } from "react";

// Generated WASM binding (`boltffi pack wasm`) + generated types & bincode
// serialization (`codegen --language typescript`). Mirrors the Leptos shell's
// `core_link.rs` (effect loop) + `http.rs` (fetch) + `kv.rs` (localStorage),
// translated to the JS FFI path a non-Rust shell must use.
import { CoreFfi } from "shared";
import * as sharedWasm from "shared";
import { BincodeDeserializer, BincodeSerializer } from "shared_types/bincode";
import {
  HttpResponse,
  Request,
  ViewModel,
  httpErrorIo,
  httpResultErr,
  httpResultOk,
  keyValueErrorOther,
  keyValueResultErr,
  keyValueResultOk,
  matchEffect,
  serializeEvent,
  serializeHttpResult,
  serializeKeyValueResult,
  valueBytes,
  valueNone,
} from "shared_types/app";
import type {
  Effect,
  Event,
  HttpRequest,
  HttpResult,
  KeyValueOperation,
  KeyValueResult,
  Value,
} from "shared_types/app";

const wasmInitialized = (
  sharedWasm as unknown as { initialized: Promise<void> }
).initialized;

// ---- HTTP capability (mirrors `http.rs`) ----------------------------------

async function httpRequest(req: HttpRequest): Promise<HttpResult> {
  try {
    const headers = new Headers();
    for (const header of req.headers) {
      headers.append(header.name, header.value);
    }
    const response = await fetch(req.url, {
      method: req.method,
      headers,
      body: req.body.length === 0 ? undefined : new Uint8Array(req.body),
    });
    const body = new Uint8Array(await response.arrayBuffer());
    return httpResultOk(new HttpResponse(response.status, [], body));
  } catch (error) {
    return httpResultErr(httpErrorIo(String(error)));
  }
}

// ---- KeyValue capability (mirrors `kv.rs`: localStorage) ------------------

function readString(key: string): number[] | null {
  const value = window.localStorage.getItem(key);
  return value === null ? null : Array.from(new TextEncoder().encode(value));
}

function toValue(bytes: number[] | null): Value {
  return bytes === null ? valueNone() : valueBytes(bytes);
}

function kvExecute(op: KeyValueOperation): KeyValueResult {
  switch (op.kind) {
    case "Get": {
      return keyValueResultOk({ kind: "Get", value: toValue(readString(op.key)) });
    }
    case "Set": {
      const previous = toValue(readString(op.key));
      window.localStorage.setItem(
        op.key,
        new TextDecoder().decode(new Uint8Array(op.value)),
      );
      return keyValueResultOk({ kind: "Set", previous });
    }
    case "Delete": {
      const previous = toValue(readString(op.key));
      window.localStorage.removeItem(op.key);
      return keyValueResultOk({ kind: "Delete", previous });
    }
    case "Exists":
    case "ListKeys": {
      return keyValueResultErr(
        keyValueErrorOther(`unsupported kv operation: ${op.kind}`),
      );
    }
  }
}

// ---- Serialization helpers ------------------------------------------------

function serializeToBytes(
  serialize: (serializer: BincodeSerializer) => void,
): Uint8Array {
  const serializer = new BincodeSerializer();
  serialize(serializer);
  return serializer.getBytes();
}

function deserializeRequests(bytes: Uint8Array): Request[] {
  const deserializer = new BincodeDeserializer(bytes);
  const len = deserializer.deserializeLen();
  const requests: Request[] = [];
  for (let i = 0; i < len; i++) {
    requests.push(Request.deserialize(deserializer));
  }
  return requests;
}

function deserializeView(bytes: Uint8Array): ViewModel {
  return ViewModel.deserialize(new BincodeDeserializer(bytes));
}

// ---- The Core wrapper (mirrors `core_link.rs::process_effect`) ------------

export class Core {
  core: CoreFfi | null = null;
  initializing: Promise<void> | null = null;
  setState: Dispatch<SetStateAction<ViewModel | null>>;

  constructor(setState: Dispatch<SetStateAction<ViewModel | null>>) {
    this.setState = setState;
  }

  initialize(): Promise<void> {
    if (this.core) {
      return Promise.resolve();
    }
    if (!this.initializing) {
      this.initializing = wasmInitialized
        .then(() => {
          this.core = CoreFfi.new();
          this.setState(this.view());
        })
        .catch((error) => {
          this.initializing = null;
          console.error("Failed to initialize wasm core:", error);
        });
    }
    return this.initializing;
  }

  view(): ViewModel {
    if (!this.core) {
      throw new Error("Core not initialized. Call initialize() first.");
    }
    return deserializeView(this.core.view());
  }

  update(event: Event): void {
    if (!this.core) {
      throw new Error("Core not initialized. Call initialize() first.");
    }
    const bytes = serializeToBytes((s) => serializeEvent(event, s));
    const effects = this.core.update(bytes);
    for (const { id, effect } of deserializeRequests(effects)) {
      this.processEffect(id, effect);
    }
  }

  private processEffect(id: number, effect: Effect): void {
    matchEffect(effect, {
      Render: () => this.setState(this.view()),
      Http: (req) => {
        void httpRequest(req.value).then((result) => {
          this.resolve(id, serializeToBytes((s) => serializeHttpResult(result, s)));
        });
      },
      KeyValue: (op) => {
        const result = kvExecute(op.value);
        this.resolve(id, serializeToBytes((s) => serializeKeyValueResult(result, s)));
      },
    });
  }

  private resolve(id: number, data: Uint8Array): void {
    if (!this.core) {
      return;
    }
    const effects = this.core.resolve(id, data);
    for (const { id, effect } of deserializeRequests(effects)) {
      this.processEffect(id, effect);
    }
  }
}
