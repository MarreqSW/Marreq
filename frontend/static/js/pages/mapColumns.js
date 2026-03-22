export function init() {
  const form = document.getElementById('mappingForm');
  const mappingsInput = document.getElementById('columnMappings');

  if (!form || !mappingsInput) {
    return;
  }

  form.addEventListener('submit', (event) => {
    event.preventDefault();

    const mappings = [];
    form.querySelectorAll('.mapping-select').forEach((select) => {
      if (select.value) {
        mappings.push({
          excel_column: select.dataset.excelColumn,
          target_field: select.value,
        });
      }
    });

    mappingsInput.value = JSON.stringify(mappings);
    form.submit();
  });
}

