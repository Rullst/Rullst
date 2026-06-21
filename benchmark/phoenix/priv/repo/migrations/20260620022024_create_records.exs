defmodule PhoenixApp.Repo.Migrations.CreateRecords do
  use Ecto.Migration

  def change do
    create table(:records) do
      add :name, :string

      timestamps(type: :utc_datetime)
    end
  end
end
