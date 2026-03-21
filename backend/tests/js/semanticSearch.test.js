/**
 * Tests for semanticSearch.js - Semantic Search Modal functionality
 */

import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { init, _internal } from '@pages/semanticSearch.js';

// Mock dependencies
vi.mock('@core/net.js', () => ({
  jsonFetch: vi.fn(),
  postJson: vi.fn(),
}));

vi.mock('@modules/notifications.js', () => ({
  showNotification: vi.fn(),
}));

import { jsonFetch, postJson } from '@core/net.js';
import { showNotification } from '@modules/notifications.js';

describe('Semantic Search Module', () => {
  beforeEach(() => {
    document.body.innerHTML = '';
    vi.clearAllMocks();
    // Reset internal state
    _internal.state.projectId = null;
    _internal.state.enabled = false;
    _internal.state.lastQuery = '';
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Initialization', () => {
    it('should not initialize without project config', () => {
      document.body.innerHTML = '<div id="semanticSearchModal"></div>';
      
      init();
      
      expect(_internal.state.projectId).toBe(null);
    });

    it('should parse project config from page', () => {
      document.body.innerHTML = `
        <script type="application/json" id="semanticSearchConfig">
          {"projectId": 123}
        </script>
        <div id="semanticSearchModal"></div>
        <input id="semanticSearchQuery" />
        <button id="semanticSearchSubmit"></button>
      `;
      
      // Mock the status check
      jsonFetch.mockResolvedValueOnce({ embeddings_enabled: true });
      
      init();
      
      expect(_internal.state.projectId).toBe(123);
    });

    it('should check search status on init', async () => {
      document.body.innerHTML = `
        <script type="application/json" id="semanticSearchConfig">
          {"projectId": 1}
        </script>
        <div id="semanticSearchModal"></div>
        <input id="semanticSearchQuery" />
        <button id="semanticSearchSubmit"></button>
        <div id="semanticSearchDisabled" class="d-none"></div>
        <div id="semanticSearchForm"></div>
      `;
      
      jsonFetch.mockResolvedValueOnce({ embeddings_enabled: false });
      
      init();
      
      // Wait for async status check
      await new Promise(resolve => setTimeout(resolve, 10));
      
      expect(jsonFetch).toHaveBeenCalledWith(
        '/api/projects/1/requirements/semantic_search/status'
      );
    });
  });

  describe('isQuestionQuery', () => {
    it('should identify question words', () => {
      expect(_internal.isQuestionQuery('What are the safety requirements?')).toBe(true);
      expect(_internal.isQuestionQuery('How does this work?')).toBe(true);
      expect(_internal.isQuestionQuery('Why is this needed?')).toBe(true);
      expect(_internal.isQuestionQuery('Is there a constraint?')).toBe(true);
    });

    it('should identify question marks', () => {
      expect(_internal.isQuestionQuery('Tell me about constraints?')).toBe(true);
    });

    it('should not flag non-questions', () => {
      expect(_internal.isQuestionQuery('safety requirements')).toBe(false);
      expect(_internal.isQuestionQuery('REQ-001')).toBe(false);
      expect(_internal.isQuestionQuery('performance metrics')).toBe(false);
    });
  });

  describe('escapeHtml', () => {
    it('should escape HTML special characters', () => {
      expect(_internal.escapeHtml('<script>alert("xss")</script>')).toBe(
        '&lt;script&gt;alert("xss")&lt;/script&gt;'
      );
    });

    it('should handle empty strings', () => {
      expect(_internal.escapeHtml('')).toBe('');
      expect(_internal.escapeHtml(null)).toBe('');
      expect(_internal.escapeHtml(undefined)).toBe('');
    });
  });

  describe('Search Functionality', () => {
    beforeEach(() => {
      document.body.innerHTML = `
        <script type="application/json" id="semanticSearchConfig">
          {"projectId": 1}
        </script>
        <div id="semanticSearchModal"></div>
        <input id="semanticSearchQuery" value="test query" />
        <button id="semanticSearchSubmit"></button>
        <div id="semanticSearchDisabled" class="d-none"></div>
        <div id="semanticSearchForm"></div>
        <div id="semanticSearchLoading" class="d-none"></div>
        <div id="semanticSearchError" class="d-none"></div>
        <div id="semanticSearchErrorMessage"></div>
        <div id="semanticSearchResults" class="d-none"></div>
        <div id="semanticSearchResultsCount"></div>
        <div id="semanticSearchResultsList"></div>
        <div id="semanticSearchEmpty" class="d-none"></div>
        <div id="semanticSearchAnswer" class="d-none"></div>
        <div id="semanticSearchAnswerText"></div>
        <div id="semanticSearchCitations"></div>
        <select id="semanticStatusFilter"><option value="">All</option></select>
        <select id="semanticCategoryFilter"><option value="">All</option></select>
        <select id="semanticApplicabilityFilter"><option value="">All</option></select>
        <select id="semanticVerificationFilter"><option value="">All</option></select>
      `;
      
      // Mock status check
      jsonFetch.mockResolvedValueOnce({ embeddings_enabled: true });
      
      init();
    });

    it('should call API with correct parameters', async () => {
      const mockResults = {
        enabled: true,
        results: [
          {
            id: 1,
            reference_code: 'REQ-001',
            title: 'Test Requirement',
            description: 'Test description',
            snippet: 'Test...',
            score: 0.95,
            rank: 1,
            status: 'Draft',
            category: 'Functional',
            applicability: 'All',
            verification: 'Analysis',
          },
        ],
        total: 1,
      };

      jsonFetch.mockResolvedValueOnce(mockResults);

      await _internal.performSearch();

      expect(jsonFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/projects/1/requirements/semantic_search?')
      );
      expect(jsonFetch).toHaveBeenCalledWith(
        expect.stringContaining('q=test+query')
      );
    });

    it('should show empty state for no results', async () => {
      jsonFetch.mockResolvedValueOnce({
        enabled: true,
        results: [],
        total: 0,
      });

      await _internal.performSearch();

      const emptyEl = document.getElementById('semanticSearchEmpty');
      expect(emptyEl.classList.contains('d-none')).toBe(false);
    });

    it('should render results correctly', async () => {
      const mockResults = {
        enabled: true,
        results: [
          {
            id: 1,
            reference_code: 'REQ-001',
            title: 'Test Requirement',
            description: 'Description here',
            snippet: 'Snippet...',
            score: 0.95,
            rank: 1,
            status: 'Draft',
            category: 'Functional',
            applicability: 'All',
            verification: 'Analysis',
          },
          {
            id: 2,
            reference_code: 'REQ-002',
            title: 'Another Requirement',
            description: 'Another description',
            snippet: 'Another...',
            score: 0.85,
            rank: 2,
            status: 'Approved',
            category: 'Safety',
            applicability: 'All',
            verification: 'Test',
          },
        ],
        total: 2,
      };

      jsonFetch.mockResolvedValueOnce(mockResults);

      await _internal.performSearch();

      const resultsList = document.getElementById('semanticSearchResultsList');
      const items = resultsList.querySelectorAll('.list-group-item');
      
      expect(items.length).toBe(2);
      expect(items[0].textContent).toContain('REQ-001');
      expect(items[0].textContent).toContain('Test Requirement');
      expect(items[1].textContent).toContain('REQ-002');
    });

    it('should show error state on API failure', async () => {
      jsonFetch.mockRejectedValueOnce(new Error('Network error'));

      await _internal.performSearch();

      const errorEl = document.getElementById('semanticSearchError');
      const errorMessage = document.getElementById('semanticSearchErrorMessage');
      
      expect(errorEl.classList.contains('d-none')).toBe(false);
      expect(errorMessage.textContent).toContain('Network error');
    });

    it('should show notification for empty query', async () => {
      document.getElementById('semanticSearchQuery').value = '';
      
      await _internal.performSearch();
      
      expect(showNotification).toHaveBeenCalledWith(
        'Please enter a search query',
        'warning'
      );
    });
  });

  describe('RAG Answer', () => {
    beforeEach(() => {
      document.body.innerHTML = `
        <script type="application/json" id="semanticSearchConfig">
          {"projectId": 1}
        </script>
        <div id="semanticSearchModal"></div>
        <input id="semanticSearchQuery" value="What are the safety requirements?" />
        <button id="semanticSearchSubmit"></button>
        <div id="semanticSearchDisabled" class="d-none"></div>
        <div id="semanticSearchForm"></div>
        <div id="semanticSearchLoading" class="d-none"></div>
        <div id="semanticSearchError" class="d-none"></div>
        <div id="semanticSearchErrorMessage"></div>
        <div id="semanticSearchResults" class="d-none"></div>
        <div id="semanticSearchResultsCount"></div>
        <div id="semanticSearchResultsList"></div>
        <div id="semanticSearchEmpty" class="d-none"></div>
        <div id="semanticSearchAnswer" class="d-none"></div>
        <div id="semanticSearchAnswerText"></div>
        <div id="semanticSearchCitations"></div>
        <select id="semanticStatusFilter"><option value="">All</option></select>
        <select id="semanticCategoryFilter"><option value="">All</option></select>
        <select id="semanticApplicabilityFilter"><option value="">All</option></select>
        <select id="semanticVerificationFilter"><option value="">All</option></select>
      `;
      
      jsonFetch.mockResolvedValueOnce({ embeddings_enabled: true });
      init();
    });

    it('should call ask endpoint for question queries', async () => {
      const searchResponse = {
        enabled: true,
        results: [
          {
            id: 1,
            reference_code: 'REQ-SAF-001',
            title: 'Safety Requirement',
            description: 'Desc',
            snippet: 'Snippet',
            score: 0.9,
            rank: 1,
            status: 'Draft',
            category: 'Safety',
            applicability: 'All',
            verification: 'Analysis',
          },
        ],
        total: 1,
      };

      const askResponse = {
        answer: 'Based on [REQ-SAF-001], the safety requirements include...',
        citations: [
          { requirement_id: 1, reference_code: 'REQ-SAF-001', title: 'Safety Requirement' },
        ],
        results: searchResponse.results,
      };

      jsonFetch.mockResolvedValueOnce(searchResponse);
      postJson.mockResolvedValueOnce(askResponse);

      await _internal.performSearch();

      expect(postJson).toHaveBeenCalledWith(
        '/api/projects/1/requirements/ask',
        expect.objectContaining({ query: 'What are the safety requirements?' })
      );
    });
  });

  describe('Keyboard Shortcuts', () => {
    it('should open modal on Ctrl+K', () => {
      document.body.innerHTML = `
        <script type="application/json" id="semanticSearchConfig">
          {"projectId": 1}
        </script>
        <div id="semanticSearchModal"></div>
        <button id="semanticSearchBtn"></button>
        <input id="semanticSearchQuery" />
      `;
      
      jsonFetch.mockResolvedValueOnce({ embeddings_enabled: true });
      init();

      const openBtn = document.getElementById('semanticSearchBtn');
      const clickSpy = vi.spyOn(openBtn, 'click');

      const event = new KeyboardEvent('keydown', {
        key: 'k',
        ctrlKey: true,
        bubbles: true,
      });

      document.dispatchEvent(event);

      // Since bootstrap isn't available, it should fall back to clicking the button
      expect(clickSpy).toHaveBeenCalled();
    });
  });
});
