// /me/upload: there is no upload page (owner, 2026/11/22). Back to the page you came from (or home)
// with `?upload`, where the root layout opens the upload dialog ($lib/upload).
import { redirect } from '@sveltejs/kit';
import { uploadRedirect } from '$lib/upload/upload-links';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = ({ url, request }) => {
  redirect(303, uploadRedirect(url, request.headers.get('referer')));
};
