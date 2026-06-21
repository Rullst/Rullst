from django.db import models

class World(models.Model):
    id = models.AutoField(primary_key=True)
    text = models.CharField(max_length=255)

    class Meta:
        db_table = 'world'
        managed = False
