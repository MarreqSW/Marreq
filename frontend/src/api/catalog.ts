import type {
  Applicability,
  Category,
  CustomFieldDefinition,
  CustomFieldWriteBody,
  RequirementStatus,
  RequirementStatusWriteBody,
  TaggedMetadataBody,
  VerificationMethod,
  VerificationMethodWriteBody,
  VerificationStatus,
  VerificationStatusWriteBody,
} from './types';
import { fetchJson, JSON_HEADERS } from './transport';

export async function listCategories(): Promise<Category[]> {
  return fetchJson<Category[]>('/api/categories');
}

export async function createCategory(
  body: TaggedMetadataBody,
  csrfToken: string,
): Promise<{ id: number }> {
  const r = await fetchJson<{ id: number }>('/api/categories', {
    method: 'POST',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ ...body, id: body.id ?? null }),
  });
  return { id: r.id };
}

export async function updateCategory(
  id: number,
  body: TaggedMetadataBody,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/categories/${id}`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ ...body, id }),
  });
}

export async function deleteCategory(id: number, csrfToken: string): Promise<void> {
  await fetchJson(`/api/categories/${id}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}

export async function listApplicability(): Promise<Applicability[]> {
  return fetchJson<Applicability[]>('/api/applicability');
}

export async function createApplicability(
  body: TaggedMetadataBody,
  csrfToken: string,
): Promise<{ id: number }> {
  const res = await fetch('/api/applicability', {
    method: 'POST',
    credentials: 'same-origin',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ ...body, id: body.id ?? null }),
  });
  if (!res.ok) {
    const t = await res.text();
    throw new Error(t || res.statusText);
  }
  return (await res.json()) as { id: number };
}

export async function updateApplicability(
  id: number,
  body: TaggedMetadataBody,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/applicability/${id}`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ ...body, id }),
  });
}

export async function deleteApplicability(id: number, csrfToken: string): Promise<void> {
  await fetchJson(`/api/applicability/${id}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}

export async function listRequirementStatuses(): Promise<RequirementStatus[]> {
  return fetchJson<RequirementStatus[]>('/api/status');
}

export async function createRequirementStatus(
  body: RequirementStatusWriteBody,
  csrfToken: string,
): Promise<{ id: number }> {
  const res = await fetch('/api/status', {
    method: 'POST',
    credentials: 'same-origin',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({
      title: body.title,
      description: body.description,
      tag: body.tag,
      project_id: body.project_id,
      tag_color: body.tag_color ?? null,
    }),
  });
  if (!res.ok) {
    const t = await res.text();
    throw new Error(t || res.statusText);
  }
  const j = (await res.json()) as { id: number };
  return { id: j.id };
}

export async function updateRequirementStatus(
  id: number,
  body: RequirementStatusWriteBody,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/status/${id}`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({
      id,
      title: body.title,
      description: body.description,
      tag: body.tag,
      project_id: body.project_id,
      is_system: body.is_system ?? false,
      tag_color: body.tag_color ?? null,
    }),
  });
}

export async function deleteRequirementStatus(id: number, csrfToken: string): Promise<void> {
  await fetchJson(`/api/status/${id}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}

export async function listVerificationStatuses(): Promise<VerificationStatus[]> {
  return fetchJson<VerificationStatus[]>('/api/verification-status');
}

export async function createVerificationStatus(
  body: VerificationStatusWriteBody,
  csrfToken: string,
): Promise<{ id: number }> {
  const res = await fetch('/api/verification-status', {
    method: 'POST',
    credentials: 'same-origin',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({
      title: body.title,
      description: body.description,
      tag: body.tag,
      project_id: body.project_id,
      tag_color: body.tag_color ?? null,
    }),
  });
  if (!res.ok) {
    const t = await res.text();
    throw new Error(t || res.statusText);
  }
  const j = (await res.json()) as { id: number };
  return { id: j.id };
}

export async function updateVerificationStatus(
  id: number,
  body: VerificationStatusWriteBody,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/verification-status/${id}`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({
      id,
      title: body.title,
      description: body.description,
      tag: body.tag,
      project_id: body.project_id,
      is_system: body.is_system ?? false,
      tag_color: body.tag_color ?? null,
    }),
  });
}

export async function deleteVerificationStatus(id: number, csrfToken: string): Promise<void> {
  await fetchJson(`/api/verification-status/${id}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}

export async function listCustomFieldsByProject(
  projectId: number,
): Promise<CustomFieldDefinition[]> {
  return fetchJson<CustomFieldDefinition[]>(
    `/api/projects/${projectId}/custom_fields`,
  );
}

export async function createCustomField(
  projectId: number,
  body: CustomFieldWriteBody,
  csrfToken: string,
): Promise<{ id: number }> {
  const r = await fetchJson<{ id: number }>(
    `/api/projects/${projectId}/custom_fields`,
    {
      method: 'POST',
      headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
      body: JSON.stringify(body),
    },
  );
  return { id: r.id };
}

export async function updateCustomField(
  projectId: number,
  fieldId: number,
  body: CustomFieldWriteBody,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/projects/${projectId}/custom_fields/${fieldId}`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify(body),
  });
}

export async function deleteCustomField(
  projectId: number,
  fieldId: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/projects/${projectId}/custom_fields/${fieldId}`, {
    method: 'DELETE',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
  });
}

export async function listVerificationMethods(): Promise<VerificationMethod[]> {
  return fetchJson<VerificationMethod[]>('/api/verification-methods');
}

export async function createVerificationMethod(
  projectId: number,
  body: VerificationMethodWriteBody,
  csrfToken: string,
): Promise<{ id: number }> {
  const r = await fetchJson<{ id: number }>(
    `/api/projects/${projectId}/verification-methods`,
    {
      method: 'POST',
      headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
      body: JSON.stringify({ ...body, id: body.id ?? null, project_id: projectId }),
    },
  );
  return { id: r.id };
}

export async function updateVerificationMethod(
  projectId: number,
  methodId: number,
  body: VerificationMethodWriteBody,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/projects/${projectId}/verification-methods/${methodId}`, {
    method: 'PUT',
    headers: { ...JSON_HEADERS, 'X-CSRF-Token': csrfToken },
    body: JSON.stringify({ ...body, id: methodId, project_id: projectId }),
  });
}

export async function deleteVerificationMethod(
  projectId: number,
  methodId: number,
  csrfToken: string,
): Promise<void> {
  await fetchJson(`/api/projects/${projectId}/verification-methods/${methodId}`, {
    method: 'DELETE',
    headers: { 'X-CSRF-Token': csrfToken },
  });
}
