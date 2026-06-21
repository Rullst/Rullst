from django.urls import path
from bench import views

urlpatterns = [
    path('text', views.text),
    path('json', views.json),
    path('db-single', views.db_single),
    path('html', views.html),
]
