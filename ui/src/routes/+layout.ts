export const ssr = false;
export const prerender = false;

export async function load(): Promise<void> {
  if (import.meta.env.VITE_MOCK === '1') {
    const { installMocks } = await import('$lib/api/mock/install');
    installMocks();
  }
}
