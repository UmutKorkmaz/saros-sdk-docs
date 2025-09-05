// ====================================
// Saros SDK Documentation - Viewer Scripts
// ====================================

// Initialize Mermaid for diagrams
mermaid.initialize({
    startOnLoad: true,
    theme: 'dark',
    themeVariables: {
        primaryColor: '#7c3aed',
        primaryTextColor: '#fff',
        primaryBorderColor: '#6d28d9',
        lineColor: '#94a3b8',
        secondaryColor: '#1a1a2e',
        tertiaryColor: '#16213e',
        background: '#0f0f1e',
        mainBkg: '#1a1a2e',
        secondBkg: '#16213e',
        tertiaryBkg: '#0f3460'
    }
});

// Get document from URL parameter
function getDocumentPath() {
    const params = new URLSearchParams(window.location.search);
    let doc = params.get('doc');

    if (!doc) {
        doc = 'README';
    }

    // Don't add .md if already has it
    if (!doc.endsWith('.md')) {
        doc += '.md';
    }

    return doc;
}

// Update breadcrumb
function updateBreadcrumb(docPath) {
    const parts = docPath.replace('.md', '').split('/');
    const breadcrumbEl = document.getElementById('breadcrumb-current');

    if (parts.length > 1) {
        breadcrumbEl.textContent = parts.map(p =>
            p.split('-').map(word =>
                word.charAt(0).toUpperCase() + word.slice(1)
            ).join(' ')
        ).join(' › ');
    } else {
        breadcrumbEl.textContent = parts[0].split('-').map(word =>
            word.charAt(0).toUpperCase() + word.slice(1)
        ).join(' ');
    }
}

// Update active sidebar link
function updateSidebarActive(docPath) {
    const links = document.querySelectorAll('.sidebar-link');
    links.forEach(link => {
        const linkDoc = new URL(link.href).searchParams.get('doc');
        if (linkDoc && (linkDoc + '.md' === docPath || linkDoc === docPath.replace('.md', ''))) {
            link.classList.add('active');
        } else {
            link.classList.remove('active');
        }
    });
}

// Generate Table of Contents
function generateTOC() {
    const headings = document.querySelectorAll('.article h2, .article h3');
    if (headings.length === 0) return;

    const tocContent = document.getElementById('toc-content');
    const toc = document.getElementById('toc');

    tocContent.innerHTML = '';

    headings.forEach((heading, index) => {
        const id = `heading-${index}`;
        heading.id = id;

        const link = document.createElement('a');
        link.href = `#${id}`;
        link.className = `toc-link ${heading.tagName.toLowerCase()}`;
        link.textContent = heading.textContent;

        link.addEventListener('click', (e) => {
            e.preventDefault();
            heading.scrollIntoView({ behavior: 'smooth' });

            // Update active state
            document.querySelectorAll('.toc-link').forEach(l => l.classList.remove('active'));
            link.classList.add('active');
        });

        tocContent.appendChild(link);
    });

    if (headings.length > 0) {
        toc.style.display = 'block';
    }
}

// Add copy buttons to code blocks
function addCopyButtons() {
    const codeBlocks = document.querySelectorAll('pre');

    codeBlocks.forEach(block => {
        const wrapper = document.createElement('div');
        wrapper.className = 'code-block-wrapper';
        block.parentNode.insertBefore(wrapper, block);
        wrapper.appendChild(block);

        const button = document.createElement('button');
        button.className = 'copy-button';
        button.textContent = 'Copy';

        button.addEventListener('click', () => {
            const code = block.querySelector('code');
            const text = code ? code.textContent : block.textContent;

            navigator.clipboard.writeText(text).then(() => {
                button.textContent = 'Copied!';
                button.classList.add('copied');

                setTimeout(() => {
                    button.textContent = 'Copy';
                    button.classList.remove('copied');
                }, 2000);
            });
        });

        wrapper.appendChild(button);
    });
}

// Load and render markdown
async function loadDocument() {
    let docPath = getDocumentPath();
    const contentEl = document.getElementById('content');

    try {
        // Show loading state
        contentEl.innerHTML = '<div class="loading"><div class="spinner"></div></div>';

        // Build correct file path for GitHub Pages deployment
        let fetchPath;
        
        // Check if we're running on GitHub Pages
        const isGitHubPages = window.location.hostname.includes('github.io');
        
        if (isGitHubPages) {
            // For GitHub Pages, files are at root level
            fetchPath = docPath;
        } else {
            // For local development, docs folder is one level up
            fetchPath = '../docs/' + docPath;
        }

        // Remove leading slash if present
        if (fetchPath.startsWith('/')) {
            fetchPath = fetchPath.substring(1);
        }

        // Handle special cases for files that might be in different locations
        if (docPath.includes('../code-examples/')) {
            fetchPath = docPath.replace('../', '');
        }

        console.log('Attempting to fetch:', fetchPath);
        console.log('Original docPath:', docPath);
        console.log('Is GitHub Pages:', isGitHubPages);

        // Fetch markdown file with error handling
        let response;
        try {
            response = await fetch(fetchPath);
        } catch (fetchError) {
            console.error('Fetch failed:', fetchError);
            // Try alternative paths if first fetch fails
            const alternativePaths = isGitHubPages ? [
                docPath,
                `docs/${docPath}`,
                `api-reference/${docPath.replace('api-reference/', '')}`,
                `guides/${docPath.replace('guides/', '')}`,
                `tutorials/${docPath.replace('tutorials/', '')}`
            ] : [
                `docs/${docPath}`,
                `../${docPath}`,
                docPath
            ];

            for (const altPath of alternativePaths) {
                try {
                    console.log('Trying alternative path:', altPath);
                    response = await fetch(altPath);
                    if (response.ok) break;
                } catch (e) {
                    console.log('Alternative path failed:', altPath, e.message);
                }
            }
        }

        if (!response || !response.ok) {
            throw new Error(`Failed to load document: ${docPath} (Status: ${response?.status || 'Network Error'})`);
        }

        const markdown = await response.text();

        // Parse markdown to HTML
        const html = marked.parse(markdown);

        // Add metadata
        const now = new Date();
        const wordCount = markdown.split(/\s+/).length;
        const metadata = `
            <div class="article-meta">
                <div class="article-meta-item">
                    📅 ${now.toLocaleDateString()}
                </div>
                <div class="article-meta-item">
                    ⏱️ ${Math.ceil(wordCount / 200)} min read
                </div>
                <div class="article-meta-item">
                    📝 ${markdown.split('\n').length} lines
                </div>
            </div>
        `;

        // Update content
        contentEl.innerHTML = html;

        // Add metadata after the first h1 if it exists
        const firstHeading = contentEl.querySelector('h1');
        if (firstHeading) {
            firstHeading.insertAdjacentHTML('afterend', metadata);
            document.title = firstHeading.textContent + ' - Saros SDK Docs';
        } else {
            contentEl.insertAdjacentHTML('afterbegin', metadata);
        }

        // Post-processing
        updateBreadcrumb(docPath);
        updateSidebarActive(docPath);

        // Syntax highlighting
        contentEl.querySelectorAll('pre code').forEach(block => {
            hljs.highlightElement(block);
        });

        // Render mermaid diagrams
        mermaid.run();

        // Generate TOC
        generateTOC();

        // Add copy buttons
        addCopyButtons();

        // Scroll to top
        window.scrollTo(0, 0);

    } catch (error) {
        console.error('Error loading document:', error);
        contentEl.innerHTML = `
            <div class="error">
                <h2>📄 Document Not Found</h2>
                <p>${error.message}</p>
                <p>Trying to load: <code>${docPath}</code></p>
                <p>Please check the URL or navigate using the sidebar.</p>
            </div>
        `;
    }
}

// Toggle sidebar on mobile
function toggleSidebar() {
    const sidebar = document.getElementById('sidebar');
    sidebar.classList.toggle('open');
}

// Handle browser back/forward
window.addEventListener('popstate', loadDocument);

// Load document on page load
document.addEventListener('DOMContentLoaded', loadDocument);

// Update TOC active state on scroll
window.addEventListener('scroll', () => {
    const headings = document.querySelectorAll('.article h2, .article h3');
    const tocLinks = document.querySelectorAll('.toc-link');

    let current = '';
    headings.forEach(heading => {
        const rect = heading.getBoundingClientRect();
        if (rect.top < 150) {
            current = heading.id;
        }
    });

    tocLinks.forEach(link => {
        if (link.href.includes(current)) {
            link.classList.add('active');
        } else {
            link.classList.remove('active');
        }
    });
});