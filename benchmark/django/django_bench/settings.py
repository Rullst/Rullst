import os
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent

SECRET_KEY = 'bench'

DEBUG = False
ALLOWED_HOSTS = ['*']

INSTALLED_APPS = [
    'bench',
]

MIDDLEWARE = [
    'django.middleware.common.CommonMiddleware',
]

ROOT_URLCONF = 'django_bench.urls'

TEMPLATES = [
    {
        'BACKEND': 'django.template.backends.django.DjangoTemplates',
        'DIRS': [],
        'APP_DIRS': True,
    },
]

WSGI_APPLICATION = 'django_bench.wsgi.application'

DATABASES = {
    'default': {
        'ENGINE': 'django.db.backends.postgresql',
        'NAME': 'benchdb',
        'USER': 'bench',
        'PASSWORD': 'benchpassword',
        'HOST': 'postgres',
        'PORT': '5432',
    }
}
