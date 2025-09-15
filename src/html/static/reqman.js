(function () {
    const THEME_STORAGE_KEY = 'reqman-theme';
    const root = document.documentElement;

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
            /* Ignore storage errors */
        }
    }

    function applyTheme(theme) {
        if (theme === 'dark') {
            root.setAttribute('data-theme', 'dark');
        } else {
            root.removeAttribute('data-theme');
        }
    }

    function updateToggleButton(theme) {
        const toggle = document.getElementById('theme-toggle');
        if (!toggle) {
            return;
        }

        const isDark = theme === 'dark';
        const label = isDark ? 'Switch to light mode' : 'Switch to dark mode';
        toggle.dataset.theme = theme;
        toggle.setAttribute('aria-pressed', String(isDark));
        toggle.setAttribute('aria-label', label);
        toggle.setAttribute('title', label);
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

    function handleMediaChange(event) {
        if (getStoredTheme()) {
            return;
        }

        const theme = event.matches ? 'dark' : 'light';
        applyTheme(theme);
        updateToggleButton(theme);
    }

    const initialTheme = determineTheme();
    applyTheme(initialTheme);
    updateToggleButton(initialTheme);

    document.addEventListener('DOMContentLoaded', function () {
        updateToggleButton(root.getAttribute('data-theme') === 'dark' ? 'dark' : 'light');

        const toggle = document.getElementById('theme-toggle');
        if (toggle) {
            toggle.addEventListener('click', function () {
                const currentTheme = root.getAttribute('data-theme') === 'dark' ? 'dark' : 'light';
                const nextTheme = currentTheme === 'dark' ? 'light' : 'dark';
                applyTheme(nextTheme);
                persistTheme(nextTheme);
                updateToggleButton(nextTheme);
            });
        }

        if (window.matchMedia) {
            const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
            if (mediaQuery.addEventListener) {
                mediaQuery.addEventListener('change', handleMediaChange);
            } else if (mediaQuery.addListener) {
                mediaQuery.addListener(handleMediaChange);
            }
        }
    });

    function deleteRequirement(reqId, reqTitle) {
        if (
            confirm(
                `Are you sure you want to delete requirement "${reqTitle}"? This action cannot be undone.`
            )
        ) {
            fetch(`/delete_requirement/${reqId}`, {
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
                    } else if (response.status === 200) {
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
    }

    function deleteTest(testId, testName) {
        if (
            confirm(
                `Are you sure you want to delete test "${testName}"? This action cannot be undone.`
            )
        ) {
            fetch(`/delete_test/${testId}`, {
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
                    } else if (response.status === 200) {
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
    }

    window.deleteRequirement = deleteRequirement;
    window.deleteTest = deleteTest;
})();
