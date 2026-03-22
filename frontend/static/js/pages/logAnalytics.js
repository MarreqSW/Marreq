function computeAverage(element) {
  const value = Number(element.getAttribute('data-value'));
  const divisor = Number(element.getAttribute('data-divisor'));
  if (!Number.isFinite(value) || !Number.isFinite(divisor) || divisor <= 0) {
    return null;
  }
  return Math.round((value / divisor) * 100) / 100;
}

export function init() {
  document.querySelectorAll('[data-average]').forEach((element) => {
    const result = computeAverage(element);
    if (result !== null) {
      element.textContent = result;
    }
  });
}

