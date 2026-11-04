const pageDetails = {
  dashboard: {
    title: 'Your learning space',
    description: 'A little progress each day adds up to big results.',
  },
  classes: {
    title: 'My classes',
    description: 'A shared space for students and teachers to learn together.',
  },
  assignments: {
    title: 'Assignments',
    description: 'Everything you need to keep learning moving forward.',
  },
  messages: {
    title: 'Messages',
    description: 'Questions, ideas, and encouragement make learning better.',
  },
};

const pageTitle = document.querySelector('#page-title');
const pageDescription = document.querySelector('#page-description');
const breadcrumb = document.querySelector('#breadcrumb-current');
const views = document.querySelectorAll('[data-view]');
const navLinks = document.querySelectorAll('.nav-link');

function showPage(page) {
  const details = pageDetails[page] || pageDetails.dashboard;
  pageTitle.textContent = details.title;
  pageDescription.textContent = details.description;
  breadcrumb.textContent = details.title;

  views.forEach((view) => {
    view.classList.toggle('hidden', view.dataset.view !== page);
  });
  navLinks.forEach((link) => {
    const active = link.dataset.page === page;
    link.classList.toggle('active', active);
    if (active) {
      link.setAttribute('aria-current', 'page');
    } else {
      link.removeAttribute('aria-current');
    }
  });
  window.location.hash = page;
}

document.querySelectorAll('[data-page]').forEach((link) => {
  link.addEventListener('click', (event) => {
    event.preventDefault();
    showPage(link.dataset.page);
  });
});

document.querySelectorAll('[data-navigate]').forEach((button) => {
  button.addEventListener('click', () => showPage(button.dataset.navigate));
});

document.querySelectorAll('[data-role]').forEach((button) => {
  button.addEventListener('click', () => {
    const isTeacher = button.dataset.role === 'teacher';
    document.querySelectorAll('[data-role]').forEach((roleButton) => {
      const active = roleButton === button;
      roleButton.classList.toggle('active', active);
      roleButton.setAttribute('aria-pressed', String(active));
    });
    document.querySelector('#welcome-title').textContent = isTeacher
      ? 'Ready to inspire this week, Alex?'
      : 'Ready for a great week, Alex?';
    document.querySelector('#welcome-text').textContent = isTeacher
      ? 'Check in on your classrooms, share a lesson, and help every student keep moving forward.'
      : 'Your classes, assignments, and teachers are all in one place. Let’s make today count.';
    document.querySelector('#welcome-action').textContent = isTeacher
      ? 'Review assignments'
      : 'See my assignments';
    document.querySelector('.profile-copy small').textContent = isTeacher
      ? 'Teacher perspective'
      : 'Student perspective';
  });
});

document.querySelectorAll('[data-filter]').forEach((button) => {
  button.addEventListener('click', () => {
    document.querySelectorAll('[data-filter]').forEach((filterButton) => {
      const active = filterButton === button;
      filterButton.classList.toggle('active', active);
      filterButton.setAttribute('aria-pressed', String(active));
    });
    document.querySelectorAll('.assignment-row').forEach((row) => {
      const show = button.dataset.filter === 'all' || row.dataset.status === button.dataset.filter;
      row.classList.toggle('hidden', !show);
    });
  });
});

document.querySelector('#message-form').addEventListener('submit', (event) => {
  event.preventDefault();
  const input = document.querySelector('#message-input');
  const message = input.value.trim();
  if (!message) return;

  const item = document.createElement('div');
  item.className = 'message-item';
  const avatar = document.createElement('span');
  avatar.className = 'avatar avatar-purple';
  avatar.textContent = 'AM';
  const content = document.createElement('div');
  const metadata = document.createElement('div');
  metadata.className = 'message-meta';
  const sender = document.createElement('strong');
  sender.textContent = 'Alex Morgan';
  const time = document.createElement('time');
  time.textContent = 'Just now';
  const body = document.createElement('p');
  body.textContent = message;
  metadata.append(sender, time);
  content.append(metadata, body);
  item.append(avatar, content);
  document.querySelector('#message-list').append(item);
  input.value = '';
  input.focus();
});

const initialPage = window.location.hash.slice(1);
showPage(pageDetails[initialPage] ? initialPage : 'dashboard');
