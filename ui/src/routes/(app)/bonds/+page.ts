import { redirect } from '@sveltejs/kit';
import { resolve } from '$app/paths';

/** Раздел «Облигации» стал разделом «Сбережения». */
export function load(): never {
  redirect(307, resolve('/savings'));
}
