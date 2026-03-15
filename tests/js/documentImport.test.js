import { describe, expect, it } from 'vitest';
import { serializeReviewForm, syncCommitState } from '@pages/documentImport.js';

describe('document import review serialization', () => {
  it('serializes defaults and candidate overrides', () => {
    document.body.innerHTML = `
      <section data-review-root data-project-id="1" data-session-id="abc">
        <select data-default-field="reviewer_id"><option value="">Select</option><option value="3" selected>Reviewer</option></select>
        <select data-default-field="category_id"><option value="1" selected>Safety</option></select>
        <select data-default-field="applicability_id"><option value="1" selected>All</option></select>
        <select data-default-field="verification_status_id"><option value="2" selected>Pending</option></select>
        <input data-default-field="verification_source" value="sample.docx">

        <article data-candidate-kind="requirement" data-candidate-id="req-1">
          <input type="checkbox" data-field-name="include" checked>
          <input data-field-name="title" value="Imported requirement">
          <textarea data-field-name="description">The system shall log faults.</textarea>
          <input data-field-name="reference_code" value="REQ-001">
          <select data-field-name="reviewer_id"><option value="3" selected>Reviewer</option></select>
          <select data-field-name="category_id"><option value="1" selected>Safety</option></select>
          <select data-field-name="applicability_id"><option value="1" selected>All</option></select>
          <select multiple data-field-name="verification_method_ids">
            <option value="1" selected>Analysis</option>
            <option value="2">Test</option>
          </select>
          <input data-field-name="custom_field" data-custom-field-id="7" value="SIL2">
        </article>

        <article data-candidate-kind="verification" data-candidate-id="ver-1">
          <input type="checkbox" data-field-name="include" checked>
          <input data-field-name="name" value="Fault logging test">
          <textarea data-field-name="description">Verify logging.</textarea>
          <input data-field-name="reference_code" value="TEST-1">
          <input data-field-name="source" value="sample.docx">
          <select data-field-name="status_id"><option value="2" selected>Pending</option></select>
          <select data-field-name="verification_method_id"><option value="1" selected>Analysis</option></select>
        </article>

        <article data-candidate-kind="trace_link" data-candidate-id="trace-1">
          <input type="checkbox" data-field-name="include" checked>
          <input data-field-name="requirement_reference_code" value="REQ-001">
          <input data-field-name="verification_reference_code" value="TEST-1">
        </article>

        <article data-candidate-kind="requirement_link" data-candidate-id="link-1">
          <input type="checkbox" data-field-name="include">
          <input data-field-name="source_requirement_reference_code" value="REQ-001">
          <input data-field-name="target_requirement_reference_code" value="REQ-BASE-001">
          <select data-field-name="link_type"><option value="">None</option><option value="DERIVES_FROM" selected>DERIVES_FROM</option></select>
          <textarea data-field-name="rationale">Imported relation.</textarea>
        </article>
      </section>
    `;

    const root = document.querySelector('[data-review-root]');
    const payload = serializeReviewForm(root);

    expect(payload.defaults.reviewer_id).toBe(3);
    expect(payload.requirements[0].verification_method_ids).toEqual([1]);
    expect(payload.requirements[0].custom_fields).toEqual([{ field_id: 7, value: 'SIL2' }]);
    expect(payload.verifications[0].status_id).toBe(2);
    expect(payload.trace_links[0].verification_reference_code).toBe('TEST-1');
    expect(payload.requirement_links[0].include).toBe(false);
  });

  it('disables commit until the session is ready and confirmed', () => {
    document.body.innerHTML = `
      <section data-review-root data-ready-to-commit="false">
        <input type="checkbox" data-role="confirm-commit">
        <button type="button" data-role="commit-review">Commit</button>
      </section>
    `;

    const root = document.querySelector('[data-review-root]');
    const confirmInput = root.querySelector('[data-role="confirm-commit"]');
    const commitButton = root.querySelector('[data-role="commit-review"]');

    syncCommitState(root);
    expect(commitButton.disabled).toBe(true);

    root.dataset.readyToCommit = 'true';
    syncCommitState(root);
    expect(commitButton.disabled).toBe(true);

    confirmInput.checked = true;
    syncCommitState(root);
    expect(commitButton.disabled).toBe(false);
  });
});
