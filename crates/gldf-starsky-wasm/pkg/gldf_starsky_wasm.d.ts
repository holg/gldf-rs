/* tslint:disable */
/* eslint-disable */

export class StarSkyRenderer {
  free(): void;
  [Symbol.dispose](): void;
  /**
   * Get star count
   */
  star_count(): number;
  /**
   * Get stars as JSON for the info panel
   */
  get_stars_json(): string;
  /**
   * Highlight a star by name (flash effect)
   */
  highlight_star(name: string): void;
  /**
   * Load star data from localStorage
   */
  load_from_storage(key: string): number;
  /**
   * Create a new renderer attached to a canvas element
   */
  constructor(canvas_id: string);
  /**
   * Render the star sky
   */
  render(): void;
  /**
   * Resize canvas to fit parent
   */
  resize(): void;
  /**
   * Get location name
   */
  location(): string;
  /**
   * Load star data from JSON string
   */
  load_json(json: string): number;
}

/**
 * Initialize the star sky renderer and attach to window
 */
export function main(): void;

/**
 * Convenience function to render from storage
 */
export function render_from_storage(canvas_id: string, storage_key: string): StarSkyRenderer;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_starskyrenderer_free: (a: number, b: number) => void;
  readonly render_from_storage: (a: number, b: number, c: number, d: number) => [number, number, number];
  readonly starskyrenderer_get_stars_json: (a: number) => [number, number];
  readonly starskyrenderer_highlight_star: (a: number, b: number, c: number) => void;
  readonly starskyrenderer_load_from_storage: (a: number, b: number, c: number) => [number, number, number];
  readonly starskyrenderer_load_json: (a: number, b: number, c: number) => [number, number, number];
  readonly starskyrenderer_location: (a: number) => [number, number];
  readonly starskyrenderer_new: (a: number, b: number) => [number, number, number];
  readonly starskyrenderer_render: (a: number) => void;
  readonly starskyrenderer_resize: (a: number) => void;
  readonly starskyrenderer_star_count: (a: number) => number;
  readonly main: () => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_externrefs: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
