// ====================================
// Saros SDK Documentation - Home Page Scripts
// ====================================

// Page load animation
window.addEventListener('load', () => {
    setTimeout(() => {
        const loader = document.getElementById('loader');
        if (loader) {
            loader.classList.add('hidden');
        }
    }, 500);
});

// Smooth scroll for anchor links
document.querySelectorAll('a[href^="#"]').forEach(anchor => {
    anchor.addEventListener('click', function (e) {
        e.preventDefault();
        const target = document.querySelector(this.getAttribute('href'));
        if (target) {
            target.scrollIntoView({ behavior: 'smooth' });
        }
    });
});

// Intersection Observer for fade-in animations
const observerOptions = {
    threshold: 0.1,
    rootMargin: '0px 0px -50px 0px'
};

const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
        if (entry.isIntersecting) {
            entry.target.style.opacity = '1';
            entry.target.style.transform = 'translateY(0)';
        }
    });
}, observerOptions);

// Observe all animated elements
document.addEventListener('DOMContentLoaded', () => {
    // Observe feature cards and example cards
    document.querySelectorAll('.feature-card, .example-card, .stat-card').forEach(card => {
        card.style.opacity = '0';
        card.style.transform = 'translateY(20px)';
        card.style.transition = 'opacity 0.6s ease, transform 0.6s ease';
        observer.observe(card);
    });

    // Dynamic terminal typing effect
    const terminalLines = document.querySelectorAll('.terminal-line');
    terminalLines.forEach((line, index) => {
        line.style.animationDelay = `${index * 0.3}s`;
    });
});

// Add parallax effect to floating shapes
document.addEventListener('mousemove', (e) => {
    const shapes = document.querySelectorAll('.shape');
    const x = e.clientX / window.innerWidth;
    const y = e.clientY / window.innerHeight;
    
    shapes.forEach((shape, index) => {
        const speed = (index + 1) * 0.5;
        const xOffset = (x - 0.5) * speed * 50;
        const yOffset = (y - 0.5) * speed * 50;
        shape.style.transform = `translate(${xOffset}px, ${yOffset}px)`;
    });
});

// Click tracking for analytics
document.querySelectorAll('.btn, .version-badge').forEach(element => {
    element.addEventListener('click', function() {
        console.log('Navigation:', this.textContent.trim(), 'URL:', this.href || 'N/A');
    });
});

// Function to switch between TypeScript and Rust examples
function showExamples(language) {
    // Update tab states
    const tabs = document.querySelectorAll('.language-tab');
    tabs.forEach(tab => {
        tab.classList.remove('active');
        if (tab.textContent.toLowerCase().includes(language.toLowerCase()) || 
            (language === 'typescript' && tab.textContent.includes('TypeScript')) ||
            (language === 'rust' && tab.textContent.includes('Rust'))) {
            tab.classList.add('active');
        }
    });
    
    // Show/hide example grids
    const tsExamples = document.getElementById('typescript-examples');
    const rustExamples = document.getElementById('rust-examples');
    
    if (language === 'typescript') {
        tsExamples.style.display = 'grid';
        rustExamples.style.display = 'none';
    } else {
        tsExamples.style.display = 'none';
        rustExamples.style.display = 'grid';
    }
    
    // Animate the visible cards
    const visibleGrid = language === 'typescript' ? tsExamples : rustExamples;
    const cards = visibleGrid.querySelectorAll('.example-card');
    cards.forEach((card, index) => {
        card.style.opacity = '0';
        card.style.transform = 'translateY(20px)';
        setTimeout(() => {
            card.style.opacity = '1';
            card.style.transform = 'translateY(0)';
        }, 50 * index);
    });
}

// Make function globally available
window.showExamples = showExamples;