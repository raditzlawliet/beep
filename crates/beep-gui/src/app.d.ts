// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare global {
  namespace App {
    // interface Error {}
    // interface Locals {}
    // interface PageData {}
    // interface Platform {}
  }

  // Build-time constants injected by Vite define (see vite.config.js)
  interface ImportMetaEnv {
    readonly VITE_BEEP_VERSION: string;
    readonly VITE_GIT_SHA: string;
  }
}

export {};
