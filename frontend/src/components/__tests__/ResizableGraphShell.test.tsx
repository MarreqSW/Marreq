import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import ResizableGraphShell from '../graph/ResizableGraphShell';

describe('ResizableGraphShell', () => {
  it('renders children and a resize handle', () => {
    render(
      <ResizableGraphShell projectId={1} viewKey="coverage">
        <div>graph-body</div>
      </ResizableGraphShell>,
    );

    expect(screen.getByText('graph-body')).toBeInTheDocument();
    const handle = screen.getByRole('separator', { name: /resize graph height/i });
    expect(handle).toHaveAttribute('aria-orientation', 'horizontal');
    expect(handle).toHaveAttribute('aria-valuemin', '320');
    expect(handle).toHaveAttribute('aria-valuemax', '1200');
  });

  it('applies placeholder layout class when requested', () => {
    const { container } = render(
      <ResizableGraphShell projectId={2} viewKey="hierarchy" placeholder>
        <span>empty</span>
      </ResizableGraphShell>,
    );

    expect(container.querySelector('.stitch-flow')?.className).toMatch(/flex/);
  });
});
