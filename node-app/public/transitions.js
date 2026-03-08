/**
 * Global Page Transitions Script
 * Handles interception of internal links and manages the transition curtain.
 */

(function () {
    // 1. Create the curtain if it doesn't exist
    function initCurtain() {
        if (document.getElementById('transition-curtain')) return;
        const curtain = document.createElement('div');
        curtain.id = 'transition-curtain';
        // Start visible if the page just loaded to fade in
        curtain.className = 'curtain-visible';
        document.body.prepend(curtain);

        // Trigger fade in on load
        requestAnimationFrame(() => {
            setTimeout(() => {
                curtain.classList.add('curtain-hidden');
                curtain.classList.remove('curtain-visible');
                document.body.classList.add('page-transition-enter');
            }, 50);
        });
    }

    // 2. Handle navigation
    function handleNavigation(e) {
        const link = e.target.closest('a');
        if (!link) return;

        const href = link.getAttribute('href');
        const target = link.getAttribute('target');

        // Skip if:
        // - Not an internal link
        // - Opens in new tab
        // - Is a hash link or javascript:
        if (!href ||
            href.startsWith('http') && !href.includes(window.location.host) ||
            href.startsWith('#') ||
            href.startsWith('javascript:') ||
            target === '_blank' ||
            e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) {
            return;
        }

        e.preventDefault();

        const curtain = document.getElementById('transition-curtain');
        if (curtain) {
            curtain.classList.remove('curtain-hidden');
            curtain.classList.add('curtain-visible');

            // Wait for animation then navigate
            setTimeout(() => {
                window.location.href = href;
            }, 400);
        } else {
            window.location.href = href;
        }
    }

    // 3. Setup on load
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initCurtain);
    } else {
        initCurtain();
    }

    document.addEventListener('click', handleNavigation);

    // Handle back/forward cache
    window.addEventListener('pageshow', (event) => {
        if (event.persisted) {
            const curtain = document.getElementById('transition-curtain');
            if (curtain) {
                curtain.classList.add('curtain-hidden');
                curtain.classList.remove('curtain-visible');
            }
        }
    });
})();
