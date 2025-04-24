/**
 * Main JavaScript for DevRS Website with Matrix Theme
 */

document.addEventListener('DOMContentLoaded', function() {
  // Initialize code highlighting for code blocks
  if (typeof Prism !== 'undefined') {
    // Ensure Prism is loaded before highlighting
    Prism.highlightAll();
  } else {
    console.warn('Prism.js not loaded, syntax highlighting disabled.');
  }

  // Add iOS-specific class if needed
  if (/iPad|iPhone|iPod/.test(navigator.userAgent) && !window.MSStream) {
    document.documentElement.classList.add('ios-device');
  }

  // Matrix animation and effects
  initMatrixEffects();
});

// Alpine.js component for the app (handling tabs and mobile menu)
document.addEventListener('alpine:init', () => {
  Alpine.data('devApp', () => ({
    tab: 'overview', // Default tab
    mobileMenu: false,

    // Initialize from URL hash if present
    init() {
      // Check for hash in URL and set tab accordingly
      const hash = window.location.hash;
      const validTabs = ['overview', 'install', 'usage']; // Define valid tabs based on nav links
      if (hash) {
        const tabName = hash.substring(1); // Remove the # character
        if (validTabs.includes(tabName)) {
          this.tab = tabName;
        } else {
          this.tab = 'overview'; // Default to overview if hash is invalid
          window.location.hash = 'overview'; // Correct the hash
        }
      } else {
        this.tab = 'overview'; // Default if no hash
      }

      // Listen for hash changes to update tab (e.g., browser back/forward)
      window.addEventListener('hashchange', () => {
        const newHash = window.location.hash;
        if (newHash) {
          const tabName = newHash.substring(1);
          if (validTabs.includes(tabName)) {
            this.tab = tabName;
          } else {
             // If hash changes to something invalid, reset to overview
             this.tab = 'overview';
             window.location.hash = 'overview';
          }
        } else {
             this.tab = 'overview'; // Reset to overview if hash is removed
        }
      });

      // Scroll to top on page load
      window.scrollTo(0, 0);

      // Typewriter effect initialization (moved from original JS, kept for completeness)
      // Note: This effect wasn't explicitly requested to be kept, but was in the original file.
      // It might interfere with code block styling. Remove if problematic.
      // setTimeout(() => this.initTypewriterEffect(), 500);
    },

    // Change tab and update hash
    changeTab(tabName) {
      const validTabs = ['overview', 'install', 'usage'];
      if (validTabs.includes(tabName)) {
          this.tab = tabName;
          this.mobileMenu = false; // Close mobile menu on tab change
          window.location.hash = tabName; // Update URL hash
          window.scrollTo(0, 0); // Scroll to top when changing tabs

          // Re-run Prism highlighting if necessary for newly shown content
          // Needs a small delay for Alpine's x-show to take effect
          setTimeout(() => {
              if (typeof Prism !== 'undefined') {
                  Prism.highlightAll();
              }
          }, 100);

          // Typewriter effect re-trigger (Remove if not needed)
          // setTimeout(() => this.initTypewriterEffect(), 300);
      } else {
          console.warn(`Attempted to change to invalid tab: ${tabName}`);
      }
    },

    // Add typewriter effect to code blocks (Kept from original JS, remove if unused/problematic)
    initTypewriterEffect() {
      // Select only code blocks within the *currently visible* tab section
      const visibleTabContent = document.querySelector(`.tab-content:not([hidden])`);
      if (!visibleTabContent) return;

      const codeBlocks = visibleTabContent.querySelectorAll('.code-block:not(.typed)');

      codeBlocks.forEach(block => {
        if (!block.classList.contains('typed')) {
          block.classList.add('typed');

          const codeContent = block.innerHTML;
          block.innerHTML = ''; // Clear existing content

          // Create a typing container inside the code block
          const typingContainer = document.createElement('div');
          typingContainer.classList.add('typing-container'); // Optional: style this if needed
          block.appendChild(typingContainer);

          // Type out the code character by character
          let i = 0;
          const typeCode = () => {
            if (i < codeContent.length) {
              typingContainer.innerHTML += codeContent.charAt(i);
              i++;
              // Random delay for a more natural typing effect
              setTimeout(typeCode, Math.random() * 10 + 5);
            } else {
              // Once typing is done, trigger Prism highlighting for this block
              if (typeof Prism !== 'undefined') {
                 Prism.highlightElement(typingContainer.querySelector('code') || typingContainer);
              }
            }
          };

          // Start typing after a short delay
          setTimeout(typeCode, 200);
        }
      });
    }
  }));
});


// Matrix background effects initialization (Unchanged from original)
function initMatrixEffects() {
  // Fix for iOS vh issues
  function setVH() {
    let vh = window.innerHeight * 0.01;
    document.documentElement.style.setProperty('--vh', `${vh}px`);
  }

  window.addEventListener('resize', setVH);
  window.addEventListener('orientationchange', setVH);
  setVH();

  // Add occasional screen flicker effect
  const body = document.body;
  setInterval(() => {
    if (Math.random() > 0.9) {
      body.classList.add('flicker');
      setTimeout(() => {
        body.classList.remove('flicker');
      }, 150);
    }
  }, 1000);

  // Create some Rust code that float down occasionally
const rustSnippets = [
    `fn sum_even(v: &[i32]) -> i32 {
        v.iter()
         .filter(|&&x| x % 2 == 0)
         .sum()
    }`,

    `fn find_first(v: &[String]) -> Option<&str> {
        v.iter()
         .find(|s| s.starts_with("r"))
         .map(|s| s.as_str())
    }`,

    `fn parse_and_double(s: &str) -> Result<i32, std::num::ParseIntError> {
        s.trim().parse::<i32>().map(|n| n * 2)
    }`,

    `#[derive(Debug)]
    struct Config {
        retries: u8,
        verbose: bool,
    }`,

    `fn get_or_default(map: &HashMap<String, String>, key: &str) -> &str {
        map.get(key).map(|s| s.as_str()).unwrap_or("default")
    }`,

    `fn safe_div(a: f64, b: f64) -> Result<f64, &'static str> {
        if b == 0.0 {
            Err("division by zero")
        } else {
            Ok(a / b)
        }
    }`,

    `fn flatten(v: Vec<Vec<i32>>) -> Vec<i32> {
        v.into_iter().flatten().collect()
    }`,

    `fn log_lines(lines: &[&str]) {
        for (i, line) in lines.iter().enumerate() {
            println!("[{}] {}", i, line);
        }
    }`,

    `let data = std::fs::read_to_string("config.toml")
        .unwrap_or_else(|_| String::from("[default]"));`,

    `let now = std::time::Instant::now();
    do_work();
    println!("elapsed: {:?}", now.elapsed());`,

    `#[derive(Clone)]
    struct Point {
        x: f32,
        y: f32,
    }`,

    `fn open_file(path: &str) -> Result<String, std::io::Error> {
        std::fs::read_to_string(path)
    }`,

    `let nums: Vec<_> = (1..100)
        .filter(|x| x % 7 == 0)
        .take(5)
        .collect();`,

    `trait Describe {
        fn describe(&self) -> String;
    }

    impl Describe for Point {
        fn describe(&self) -> String {
            format!("({}, {})", self.x, self.y)
        }
    }`,

    `async fn fetch_url(url: &str) -> Result<String, reqwest::Error> {
        reqwest::get(url).await?.text().await
    }`,

    `struct Holder<'a> {
        name: &'a str,
    }

    impl<'a> Holder<'a> {
        fn greet(&self) {
            println!("Hello, {}", self.name);
        }
    }`,

    `fn retry<F, T, E>(mut f: F, max: u8) -> Result<T, E>
    where
        F: FnMut() -> Result<T, E>,
    {
        for _ in 0..max {
            if let Ok(val) = f() {
                return Ok(val);
            }
        }
        f()
    }`,

    `let result = match env::var("MODE") {
        Ok(val) if val == "debug" => "debugging",
        Ok(_) => "running",
        Err(_) => "unknown",
    };`,

    `let pool = Arc::new(Mutex::new(vec![]));
    {
        let mut conn = pool.lock().unwrap();
        conn.push(42);
    }`,

    `#[repr(C)]
    pub struct FFIData {
        pub id: u32,
        pub value: f64,
    }`,

    `fn map_result(v: Vec<&str>) -> Vec<Result<i32, _>> {
        v.into_iter().map(|s| s.parse()).collect()
    }`,

    `fn zip_and_sum(a: &[i32], b: &[i32]) -> i32 {
        a.iter().zip(b.iter()).map(|(x, y)| x + y).sum()
    }`,

    `impl Config {
        fn is_enabled(&self) -> bool {
            self.verbose && self.retries > 0
        }
    }`,

    `fn dedup_and_sort(mut v: Vec<i32>) -> Vec<i32> {
        v.sort_unstable();
        v.dedup();
        v
    }`,

    `async fn async_read(path: &str) -> std::io::Result<String> {
        use tokio::fs::File;
        use tokio::io::AsyncReadExt;
        let mut file = File::open(path).await?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).await?;
        Ok(contents)
    }`,

    `fn partition(v: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        v.into_iter().partition(|x| x % 2 == 0)
    }`,

    `fn unwrap_or_log(opt: Option<i32>) -> i32 {
        opt.unwrap_or_else(|| {
            eprintln!("Missing value, using default");
            0
        })
    }`,

    `fn to_map(pairs: Vec<(&str, i32)>) -> HashMap<String, i32> {
        pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }`,

    `#[async_trait]
    trait Syncable {
        async fn sync(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    }`,

    `fn split_words(s: &str) -> Vec<&str> {
        s.split_whitespace().collect()
    }`
    ];

  function createMatrixChar() {
      const char = document.createElement('div');
      char.innerText = rustSnippets[Math.floor(Math.random() * rustSnippets.length)];
      char.style.whiteSpace = 'pre';
      char.style.position = 'fixed';
      char.style.color = 'var(--matrix-green)';
      char.style.fontSize = (8 + Math.random() * 8).toFixed(1) + 'px';
      char.style.opacity = '0.15';
      char.style.left = (Math.random() * 70) + 'vw';
      char.style.top = '0vh';
      char.style.zIndex = '-1';
      char.style.textShadow = '0 0 3px rgba(0, 255, 65, 0.5)';
      document.body.appendChild(char);

      // Animate downward like in The Matrix
      let pos = 0;
      const speed = 0.005 + (Math.random() * 0.1); // Random speeds
      const animate = () => {
        pos += speed;
        char.style.top = pos + 'vh';

        if (pos < 120) {
          requestAnimationFrame(animate);
        } else {
          char.remove();
        }
      };

      requestAnimationFrame(animate);
  }

  // Create floating code occasionally
  setInterval(() => {
    // Create 0-3 characters at random positions
    const count = Math.floor(Math.random() * 2);
    for (let i = 0; i < count; i++) {
      createMatrixChar();
    }
  }, 500);

  // Add a subtle pulsing effect to the matrix background
  const matrixBg = document.querySelector('.matrix-bg');
  if (matrixBg) {
    // Add random subtle changes to opacity
    setInterval(() => {
      matrixBg.style.opacity = (0.05 + Math.random() * 0.04).toString();
    }, 2000);
  }
}

// Fix for Safari 100vh fix
if (navigator.userAgent.indexOf('Safari') != -1 &&
    navigator.userAgent.indexOf('Chrome') == -1) {
  document.documentElement.classList.add('safari');

  const updateHeight = () => {
    document.documentElement.style.setProperty(
      '--real-vh',
      `${window.innerHeight * 0.01}px`
    );
  };

  window.addEventListener('resize', updateHeight);
  updateHeight();
}

// Improve scrolling performance
let ticking = false;
window.addEventListener('scroll', function() {
  if (!ticking) {
    window.requestAnimationFrame(function() {
      ticking = false;
    });
    ticking = true;
  }
}, {passive: true});

// iOS-specific touch improvements
if (/iPad|iPhone|iPod/.test(navigator.userAgent) && !window.MSStream) {
  document.addEventListener('touchstart', function(){}, {passive: true});
}
