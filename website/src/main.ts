import './style.css'

// Navbar scroll effect
const navbar = document.querySelector('.navbar');

window.addEventListener('scroll', () => {
  if (window.scrollY > 50) {
    navbar?.classList.add('scrolled');
  } else {
    navbar?.classList.remove('scrolled');
  }
});

// Copy to clipboard functionality
const copyBtn = document.getElementById('copy-install');

copyBtn?.addEventListener('click', async () => {
  try {
    await navigator.clipboard.writeText('cargo install cargo-rullst');
    const originalText = copyBtn.innerText;
    copyBtn.innerText = '✓ Copied!';
    copyBtn.classList.add('copied');
    
    setTimeout(() => {
      copyBtn.innerText = originalText;
      copyBtn.classList.remove('copied');
    }, 2000);
  } catch (err) {
    console.error('Failed to copy text: ', err);
  }
});
