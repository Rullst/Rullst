defmodule PhoenixApp.Record do
  use Ecto.Schema
  import Ecto.Changeset

  schema "records" do
    field :name, :string

    timestamps(type: :utc_datetime)
  end

  @doc false
  def changeset(record, attrs) do
    record
    |> cast(attrs, [:name])
    |> validate_required([:name])
  end
end
