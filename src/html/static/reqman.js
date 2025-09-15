(function () {
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
