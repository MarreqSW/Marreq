(function () {
    'use strict';

    const root = document.documentElement;
    const THEME_STORAGE_KEY = 'reqman-theme';
    const SIDEBAR_STORAGE_KEY = 'reqman_sidebar_collapsed';

    function getStoredTheme() {
        try {
            return localStorage.getItem(THEME_STORAGE_KEY);
        } catch (error) {
            return null;
        }
    }

    function persistTheme(theme) {
        try {
            localStorage.setItem(THEME_STORAGE_KEY, theme);
        } catch (error) {
            /* ignore */
        }
    }

    function applyTheme(theme) {
        if (theme === 'dark') {
            root.setAttribute('data-theme', 'dark');
        } else {
            root.removeAttribute('data-theme');
        }
    }

    function determineTheme() {
        const stored = getStoredTheme();
        if (stored === 'dark' || stored === 'light') {
            return stored;
        }

        if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
            return 'dark';
        }

        return 'light';
    }

    function updateToggleButtons(theme) {
        const isDark = theme === 'dark';
        const label = isDark ? 'Switch to light mode' : 'Switch to dark mode';

        document.querySelectorAll('[data-theme-toggle]').forEach((toggle) => {
            toggle.setAttribute('aria-pressed', String(isDark));
            toggle.setAttribute('aria-label', label);
            toggle.setAttribute('title', label);
        });
    }

    function handleThemeMediaChange(event) {
        if (getStoredTheme()) {
            return;
        }

        const theme = event.matches ? 'dark' : 'light';
        applyTheme(theme);
        updateToggleButtons(theme);
    }

    const initialTheme = determineTheme();
    applyTheme(initialTheme);
    updateToggleButtons(initialTheme);

    document.addEventListener('DOMContentLoaded', () => {
        initThemeToggle();
        initSidebar();
        initRequirementTable();
        initTestTable();
        initRequirementModal();
        initTestModal();
        initDeleteRequirementButtons();
        initDeleteTestButtons();
        initDeleteProjectButtons();
        initProjectSelector();
    });

    function initThemeToggle() {
        const toggles = document.querySelectorAll('[data-theme-toggle]');
        const currentTheme = root.getAttribute('data-theme') === 'dark' ? 'dark' : 'light';
        updateToggleButtons(currentTheme);

        toggles.forEach((toggle) => {
            toggle.addEventListener('click', () => {
                const activeTheme = root.getAttribute('data-theme') === 'dark' ? 'dark' : 'light';
                const nextTheme = activeTheme === 'dark' ? 'light' : 'dark';
                applyTheme(nextTheme);
                persistTheme(nextTheme);
                updateToggleButtons(nextTheme);
            });
        });

        if (window.matchMedia) {
            const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
            if (mediaQuery.addEventListener) {
                mediaQuery.addEventListener('change', handleThemeMediaChange);
            } else if (mediaQuery.addListener) {
                mediaQuery.addListener(handleThemeMediaChange);
            }
        }
    }

    function initSidebar() {
        const sidebar = document.getElementById('mainSidebar');
        if (!sidebar) {
            return;
        }

        const sidebarToggle = document.getElementById('sidebarToggle');
        const mobileToggle = document.getElementById('mobileToggle');
        const isDesktop = () => window.innerWidth >= 992;

        if (localStorage.getItem(SIDEBAR_STORAGE_KEY) === 'true' && isDesktop()) {
            sidebar.classList.add('reqman-sidebar--collapsed');
        }

        const toggleSidebar = () => {
            const collapsed = sidebar.classList.toggle('reqman-sidebar--collapsed');
            if (isDesktop()) {
                try {
                    localStorage.setItem(SIDEBAR_STORAGE_KEY, String(collapsed));
                } catch (error) {
                    /* ignore */
                }
            }
        };

        if (sidebarToggle) {
            sidebarToggle.addEventListener('click', toggleSidebar);
        }

        if (mobileToggle) {
            mobileToggle.addEventListener('click', () => {
                sidebar.classList.toggle('reqman-sidebar--mobile-open');
            });
        }

        document.addEventListener('click', (event) => {
            if (
                window.innerWidth < 992 &&
                !sidebar.contains(event.target) &&
                (!mobileToggle || !mobileToggle.contains(event.target)) &&
                sidebar.classList.contains('reqman-sidebar--mobile-open')
            ) {
                sidebar.classList.remove('reqman-sidebar--mobile-open');
            }
        });
    }

    function initProjectSelector() {
        const selector = document.getElementById('project-selector');
        if (!selector) {
            return;
        }

        selector.addEventListener('change', () => {
            const projectId = selector.value;
            if (projectId) {
                changeProject(projectId);
            }
        });

        const hasProjectCookie = document.cookie
            .split(';')
            .map((cookie) => cookie.trim())
            .some((cookie) => cookie.startsWith('selected_project_id='));

        if (!hasProjectCookie) {
            const firstOption = selector.querySelector('option[value]');
            if (firstOption) {
                selector.value = firstOption.value;
                changeProject(firstOption.value);
            }
        }
    }

    function changeProject(projectId) {
        if (!projectId) {
            return;
        }

        document.cookie = `selected_project_id=${projectId}; path=/; max-age=86400`;

        const path = window.location.pathname;
        const segments = path.split('/').filter(Boolean);

        if (segments[0] === 'p' && segments.length >= 2) {
            segments[1] = projectId;
            const newPath = `/${segments.join('/')}`;
            const suffix = window.location.search + window.location.hash;
            window.location.assign(`${newPath}${suffix}`);
        } else {
            window.location.reload();
        }
    }

    function initDeleteRequirementButtons() {
        document.querySelectorAll('.js-delete-requirement').forEach((button) => {
            button.addEventListener('click', () => {
                const projectId = Number(button.getAttribute('data-project-id'));
                const requirementId = Number(button.getAttribute('data-requirement-id'));
                const title = button.getAttribute('data-requirement-title') || 'Requirement';
                deleteRequirement(projectId, requirementId, title);
            });
        });
    }

    function initDeleteTestButtons() {
        document.querySelectorAll('.js-delete-test').forEach((button) => {
            button.addEventListener('click', () => {
                const projectId = Number(button.getAttribute('data-project-id'));
                const testId = Number(button.getAttribute('data-test-id'));
                const name = button.getAttribute('data-test-name') || 'Test';
                deleteTest(projectId, testId, name);
            });
        });
    }

    function initDeleteProjectButtons() {
        document.querySelectorAll('.js-delete-project').forEach((button) => {
            button.addEventListener('click', () => {
                const projectId = Number(button.getAttribute('data-project-id'));
                const name = button.getAttribute('data-project-name') || 'this project';
                deleteProject(projectId, name);
            });
        });
    }

    function getActiveProjectId(explicitProjectId) {
        if (explicitProjectId !== undefined && explicitProjectId !== null) {
            return explicitProjectId;
        }

        const pathMatch = window.location.pathname.match(/^\/p\/(\d+)/);
        if (pathMatch) {
            return Number(pathMatch[1]);
        }

        const projectCookie = document.cookie
            .split(';')
            .map((cookie) => cookie.trim())
            .find((cookie) => cookie.startsWith('selected_project_id='));

        if (projectCookie) {
            const value = projectCookie.split('=')[1];
            const parsed = Number(value);
            if (!Number.isNaN(parsed)) {
                return parsed;
            }
        }

        return null;
    }

    function deleteRequirement(projectId, requirementId, title) {
        const resolvedProjectId = getActiveProjectId(projectId);
        if (resolvedProjectId === null) {
            alert('Unable to determine the current project. Please reload the page and try again.');
            return;
        }

        if (
            !confirm(
                `Are you sure you want to delete requirement "${title}"? This action cannot be undone.`
            )
        ) {
            return;
        }

        fetch(`/p/${resolvedProjectId}/requirements/delete/${requirementId}`, {
            method: 'DELETE',
            headers: {
                'Content-Type': 'application/json',
            },
        })
            .then((response) => {
                if (response.redirected) {
                    window.location.href = response.url;
                } else if (response.status === 403) {
                    alert(
                        'Access denied. Only admin users can delete requirements with status other than Draft or Proposal.'
                    );
                } else if (response.ok) {
                    window.location.reload();
                } else {
                    alert('Error deleting requirement. Please try again.');
                }
            })
            .catch((error) => {
                console.error('Error:', error);
                alert('Error deleting requirement. Please try again.');
            });
    }

    function deleteTest(projectId, testId, name) {
        const resolvedProjectId = getActiveProjectId(projectId);
        if (resolvedProjectId === null) {
            alert('Unable to determine the current project. Please reload the page and try again.');
            return;
        }

        if (!confirm(`Are you sure you want to delete test "${name}"? This action cannot be undone.`)) {
            return;
        }

        fetch(`/p/${resolvedProjectId}/tests/delete/${testId}`, {
            method: 'DELETE',
            headers: {
                'Content-Type': 'application/json',
            },
        })
            .then((response) => {
                if (response.redirected) {
                    window.location.href = response.url;
                } else if (response.status === 403) {
                    alert(
                        'Access denied. Only admin users can delete tests with status other than Draft or Proposal.'
                    );
                } else if (response.ok) {
                    window.location.reload();
                } else {
                    alert('Error deleting test. Please try again.');
                }
            })
            .catch((error) => {
                console.error('Error:', error);
                alert('Error deleting test. Please try again.');
            });
    }

    function deleteProject(projectId, projectName) {
        if (!projectId) {
            return;
        }

        if (
            !confirm(
                `Are you sure you want to delete project "${projectName}"? This action cannot be undone.`
            )
        ) {
            return;
        }

        fetch(`/p/${projectId}/delete`, {
            method: 'DELETE',
        })
            .then((response) => {
                if (response.ok) {
                    window.location.reload();
                } else {
                    alert('Error deleting project');
                }
            })
            .catch((error) => {
                console.error('Error:', error);
                alert('Error deleting project');
            });
    }

    function initTableSorting(table, columnMap) {
        if (!table) {
            return;
        }

        const state = { column: null, order: 'asc' };

        const getCellValue = (row, columnKey) => {
            const index = columnMap[columnKey];
            const cell = row.cells[index];
            if (!cell) {
                return '';
            }

            const input = cell.querySelector('input');
            const select = cell.querySelector('select');
            const span = cell.querySelector('span');

            if (input) {
                return input.value;
            }

            if (select) {
                const option = select.options[select.selectedIndex];
                return option ? option.text : '';
            }

            if (span) {
                return span.textContent;
            }

            return cell.textContent || '';
        };

        table.addEventListener('click', (event) => {
            const trigger = event.target.closest('.sort-trigger');
            if (!trigger || !table.contains(trigger)) {
                return;
            }

            event.preventDefault();
            const columnKey = trigger.getAttribute('data-sort-key');
            if (!columnKey || !(columnKey in columnMap)) {
                return;
            }

            if (state.column === columnKey) {
                state.order = state.order === 'asc' ? 'desc' : 'asc';
            } else {
                state.column = columnKey;
                state.order = 'asc';
            }

            const rows = Array.from(table.querySelectorAll('tbody tr'));
            rows.sort((a, b) => {
                const aValue = getCellValue(a, columnKey).toLowerCase();
                const bValue = getCellValue(b, columnKey).toLowerCase();

                if (aValue < bValue) {
                    return state.order === 'asc' ? -1 : 1;
                }
                if (aValue > bValue) {
                    return state.order === 'asc' ? 1 : -1;
                }
                return 0;
            });

            const tbody = table.querySelector('tbody');
            rows.forEach((row) => tbody.appendChild(row));

            updateSortIndicators(table, columnKey, state.order);
        });
    }

    function updateSortIndicators(table, activeKey, order) {
        table.querySelectorAll('.sort-trigger .sort-indicator').forEach((indicator) => {
            indicator.textContent = '↕';
        });

        const activeIndicator = table.querySelector(
            `.sort-trigger[data-sort-key="${activeKey}"] .sort-indicator`
        );
        if (activeIndicator) {
            activeIndicator.textContent = order === 'asc' ? '↑' : '↓';
        }
    }

    function initInlineTextEditing(container, selector, onCommit) {
        container.querySelectorAll(selector).forEach((element) => {
            element.addEventListener('click', () => {
                if (element.querySelector('input')) {
                    return;
                }

                const original = element.textContent.trim();
                const field = element.getAttribute('data-field');
                const id = element.getAttribute('data-id');
                if (!field || !id) {
                    return;
                }

                const input = document.createElement('input');
                input.type = 'text';
                input.className = 'form-control form-control-sm';
                input.value = original;

                element.textContent = '';
                element.appendChild(input);
                input.focus();
                input.select();

                const revert = () => {
                    element.textContent = original;
                };

                const commit = () => {
                    const nextValue = input.value.trim();
                    element.textContent = nextValue;
                    if (nextValue === original) {
                        return;
                    }
                    onCommit({ id, field, value: nextValue, revert });
                };

                input.addEventListener('blur', commit);
                input.addEventListener('keydown', (event) => {
                    if (event.key === 'Enter') {
                        event.preventDefault();
                        commit();
                    } else if (event.key === 'Escape') {
                        event.preventDefault();
                        revert();
                    }
                });
            });
        });
    }

    function initInlineChangeHandling(container, selector, onChange) {
        container.querySelectorAll(selector).forEach((element) => {
            element.addEventListener('change', () => {
                const field = element.getAttribute('data-field');
                const id = element.getAttribute('data-id');
                if (!field || !id) {
                    return;
                }
                onChange({ id, field, value: element.value });
            });
        });
    }

    function initRequirementTable() {
        const table = document.getElementById('requirementsTable');
        if (!table) {
            return;
        }

        initTableSorting(table, {
            req_id: 0,
            req_title: 1,
            req_reference: 2,
            req_category: 3,
            req_current_status: 4,
            req_verification: 5,
            req_author: 6,
            req_reviewer: 7,
            req_creation_date: 8,
            req_deadline_date: 9,
        });

        initInlineTextEditing(table, '.editable-field', ({ id, field, value, revert }) => {
            updateRequirementField(id, field, value)
                .then(() => {
                    showNotification('Requirement updated successfully', 'success');
                })
                .catch((error) => {
                    console.error(error);
                    showNotification('Error updating requirement', 'error');
                    revert();
                });
        });

        initInlineChangeHandling(table, '.editable-select', ({ id, field, value }) => {
            updateRequirementField(id, field, value)
                .then(() => {
                    showNotification('Requirement updated successfully', 'success');
                })
                .catch((error) => {
                    console.error(error);
                    showNotification('Error updating requirement', 'error');
                });
        });

        initInlineChangeHandling(table, '.editable-date', ({ id, field, value }) => {
            updateRequirementField(id, field, value)
                .then(() => {
                    showNotification('Requirement updated successfully', 'success');
                })
                .catch((error) => {
                    console.error(error);
                    showNotification('Error updating requirement', 'error');
                });
        });
    }

    function initTestTable() {
        const table = document.getElementById('testsTable');
        if (!table) {
            return;
        }

        initTableSorting(table, {
            test_id: 0,
            test_name: 1,
            test_reference: 2,
            test_description: 3,
            test_status: 4,
            test_source: 5,
            test_parent: 6,
        });

        initInlineTextEditing(table, '.editable-field', ({ id, field, value, revert }) => {
            updateTestField(id, field, value)
                .then(() => {
                    showNotification('Test updated successfully', 'success');
                })
                .catch((error) => {
                    console.error(error);
                    showNotification('Error updating test', 'error');
                    revert();
                });
        });

        initInlineChangeHandling(table, '.editable-select', ({ id, field, value }) => {
            updateTestField(id, field, value)
                .then(() => {
                    showNotification('Test updated successfully', 'success');
                })
                .catch((error) => {
                    console.error(error);
                    showNotification('Error updating test', 'error');
                });
        });
    }

    function updateRequirementField(id, field, rawValue) {
        const numericFields = new Set([
            'req_current_status',
            'req_verification',
            'req_author',
            'req_reviewer',
            'req_category',
            'req_applicability',
        ]);

        const payload = {};
        if (numericFields.has(field)) {
            const numericValue = Number(rawValue);
            if (Number.isNaN(numericValue)) {
                return Promise.reject(new Error('Invalid numeric value'));
            }
            payload[field] = numericValue;
        } else {
            payload[field] = rawValue;
        }

        return fetch(`/api/v1/requirements/${id}`, {
            method: 'PATCH',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify(payload),
        }).then((response) =>
            response.json().then((data) => {
                if (!response.ok || !data.success) {
                    throw new Error(data.message || 'Unable to update requirement');
                }
            })
        );
    }

    function updateTestField(id, field, value) {
        return fetch(`/api/v1/tests/${id}/field`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ field, value }),
        }).then((response) =>
            response.json().then((data) => {
                if (!response.ok || !data.success) {
                    throw new Error(data.message || 'Unable to update test');
                }
            })
        );
    }

    function initRequirementModal() {
        const trigger = document.getElementById('addNewRequirement');
        const modalElement = document.getElementById('addRequirementModal');
        const form = document.getElementById('addRequirementForm');

        if (!trigger || !modalElement || !form || !window.bootstrap) {
            return;
        }

        const modal = new window.bootstrap.Modal(modalElement);

        trigger.addEventListener('click', () => {
            modal.show();
        });

        form.addEventListener('submit', (event) => {
            event.preventDefault();
            const formData = new FormData(form);
            const payload = Object.fromEntries(formData.entries());

            fetch('/api/v1/requirements', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(payload),
            })
                .then((response) => response.json())
                .then((data) => {
                    if (data.success) {
                        showNotification('Requirement added successfully', 'success');
                        modal.hide();
                        setTimeout(() => window.location.reload(), 600);
                    } else {
                        showNotification('Error adding requirement: ' + (data.message || ''), 'error');
                    }
                })
                .catch((error) => {
                    console.error(error);
                    showNotification('Error adding requirement', 'error');
                });
        });
    }

    function initTestModal() {
        const trigger = document.getElementById('addNewTest');
        const modalElement = document.getElementById('addTestModal');
        const form = document.getElementById('addTestForm');

        if (!trigger || !modalElement || !form || !window.bootstrap) {
            return;
        }

        const modal = new window.bootstrap.Modal(modalElement);

        trigger.addEventListener('click', () => {
            modal.show();
        });

        form.addEventListener('submit', (event) => {
            event.preventDefault();
            const formData = new FormData(form);
            const payload = Object.fromEntries(formData.entries());

            fetch('/api/v1/tests', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(payload),
            })
                .then((response) => response.json())
                .then((data) => {
                    if (data.success) {
                        showNotification('Test added successfully', 'success');
                        modal.hide();
                        setTimeout(() => window.location.reload(), 600);
                    } else {
                        showNotification('Error adding test: ' + (data.message || ''), 'error');
                    }
                })
                .catch((error) => {
                    console.error(error);
                    showNotification('Error adding test', 'error');
                });
        });
    }

    function showNotification(message, type) {
        const wrapper = document.createElement('div');
        const alertClass = type === 'success' ? 'alert-success' : 'alert-danger';
        wrapper.className = `alert ${alertClass} alert-dismissible fade show position-fixed`;
        wrapper.style.top = '20px';
        wrapper.style.right = '20px';
        wrapper.style.zIndex = '9999';
        wrapper.innerHTML = `
            ${message}
            <button type="button" class="btn-close" data-bs-dismiss="alert" aria-label="Close"></button>
        `;

        document.body.appendChild(wrapper);
        setTimeout(() => {
            wrapper.classList.remove('show');
            wrapper.addEventListener('transitionend', () => wrapper.remove(), { once: true });
        }, 3000);
    }
})();
