document.addEventListener('DOMContentLoaded', () => {
    // Copy to clipboard functionality
    const copyBtn = document.getElementById('copy-btn');
    
    if (copyBtn) {
        copyBtn.addEventListener('click', async () => {
            const textToCopy = 'cargo add rullst-connect';
            try {
                await navigator.clipboard.writeText(textToCopy);
                copyBtn.classList.add('copied');
                
                // Remove the 'copied' class after 2 seconds
                setTimeout(() => {
                    copyBtn.classList.remove('copied');
                }, 2000);
            } catch (err) {
                console.error('Failed to copy text: ', err);
            }
        });
    }

    // Intersection Observer for scroll animations
    const observerOptions = {
        root: null,
        rootMargin: '0px',
        threshold: 0.1
    };

    const observer = new IntersectionObserver((entries, observer) => {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                entry.target.style.opacity = '1';
                entry.target.style.transform = 'translateY(0)';
                observer.unobserve(entry.target);
            }
        });
    }, observerOptions);

    // Apply animation starting styles and observe elements
    const animatedElements = document.querySelectorAll('.feature-card, .provider-tag, .code-showcase');
    
    animatedElements.forEach((el, index) => {
        el.style.opacity = '0';
        el.style.transform = 'translateY(20px)';
        el.style.transition = `opacity 0.6s ease, transform 0.6s ease`;
        
        // Stagger delays for grid items
        if (el.classList.contains('provider-tag')) {
            el.style.transitionDelay = `${(index % 10) * 0.05}s`;
        } else if (el.classList.contains('feature-card')) {
            el.style.transitionDelay = `${(index % 4) * 0.1}s`;
        }
        
        observer.observe(el);
    });

    // Smooth scroll for anchor links
    document.querySelectorAll('a[href^="#"]').forEach(anchor => {
        anchor.addEventListener('click', function (e) {
            e.preventDefault();
            const targetId = this.getAttribute('href');
            if (targetId === '#') return;
            
            const targetElement = document.querySelector(targetId);
            if (targetElement) {
                targetElement.scrollIntoView({
                    behavior: 'smooth'
                });
            }
        });
    });
});
