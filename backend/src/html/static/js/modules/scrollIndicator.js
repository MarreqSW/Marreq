export function initScrollIndicator({
  containerSelector,
  indicatorSelector,
  thumbSelector,
}) {
  const container = document.querySelector(containerSelector);
  const indicator = document.querySelector(indicatorSelector);
  const thumb = document.querySelector(thumbSelector);

  if (!container || !indicator || !thumb) {
    return;
  }

  let maxScroll = 0;
  let thumbMaxLeft = 0;
  let isDragging = false;
  let startX = 0;
  let startLeft = 0;

  const updateThumbPosition = () => {
    if (maxScroll <= 0) return;
    const scrollPercent = Math.min(1, Math.max(0, container.scrollLeft / maxScroll));
    const thumbLeft = scrollPercent * thumbMaxLeft;
    thumb.style.left = `${thumbLeft}px`;
  };

  function updateIndicator() {
    const tableWidth = container.scrollWidth;
    const containerWidth = container.clientWidth;

    // Mark as initialized to show with fade-in
    indicator.classList.add('is-initialized');

    if (tableWidth > containerWidth) {
      indicator.style.display = 'block';
      maxScroll = tableWidth - containerWidth;
      const thumbWidth = Math.max(30, (containerWidth / tableWidth) * containerWidth);
      thumbMaxLeft = containerWidth - thumbWidth - 8;
      thumb.style.width = `${thumbWidth}px`;
      updateThumbPosition();
    } else {
      indicator.style.display = 'none';
    }
  }

  container.addEventListener('scroll', updateThumbPosition);

  thumb.addEventListener('mousedown', (event) => {
    isDragging = true;
    startX = event.clientX;
    startLeft = parseFloat(thumb.style.left) || 0;
    event.preventDefault();
    event.stopPropagation();
  });

  document.addEventListener('mousemove', (event) => {
    if (!isDragging) return;
    const delta = event.clientX - startX;
    const newLeft = Math.max(0, Math.min(thumbMaxLeft, startLeft + delta));
    thumb.style.left = `${newLeft}px`;
    const scrollPercent = thumbMaxLeft ? newLeft / thumbMaxLeft : 0;
    container.scrollLeft = scrollPercent * maxScroll;
  });

  document.addEventListener('mouseup', () => {
    isDragging = false;
  });

  indicator.addEventListener('click', (event) => {
    if (event.target === thumb || isDragging) return;
    const rect = indicator.getBoundingClientRect();
    const clickX = event.clientX - rect.left - 4;
    const trackWidth = rect.width - 8;
    const clickPercent = Math.max(0, Math.min(1, clickX / trackWidth));
    container.scrollLeft = clickPercent * maxScroll;
  });

  const observer = new MutationObserver(() => {
    updateIndicator();
  });

  observer.observe(container, { childList: true, subtree: true });

  window.addEventListener('resize', updateIndicator);
  setTimeout(updateIndicator, 100);
}

