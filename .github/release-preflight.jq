def fail($message): error("release preflight: " + $message);

($release_order[0]) as $order
| (.packages | map(select(.publish != []))) as $publishable
| ($publishable | map(.name)) as $actual_names
| (reduce range(0; $order | length) as $index
    ({}; .[$order[$index]] = $index)) as $positions
| if ($order | length) != ($order | unique | length) then
    fail("release-order.json contains duplicate package names")
  elif ($actual_names | sort) != ($order | sort) then
    fail(
      "release-order.json does not match the publishable workspace packages; expected="
      + (($actual_names | sort) | join(","))
      + "; configured="
      + (($order | sort) | join(","))
    )
  elif any($publishable[]; .version != $version) then
    fail(
      "tag version does not match: "
      + ([$publishable[] | select(.version != $version) | (.name + "=" + .version)]
        | join(","))
    )
  elif any(
    $publishable[] as $package
    | $package.dependencies[]
    | select($positions[.name] != null)
    | {package: $package, dependency: .};
    .dependency.path == null
  ) then
    fail("an internal dependency is missing its workspace path")
  elif any(
    $publishable[] as $package
    | $package.dependencies[]
    | select($positions[.name] != null)
    | {package: $package, dependency: .};
    .dependency.req != ("^" + $version)
  ) then
    fail("an internal dependency requirement does not match ^" + $version)
  elif any(
    $publishable[] as $package
    | $package.dependencies[]
    | select($positions[.name] != null)
    | {package: $package, dependency: .};
    $positions[.dependency.name] >= $positions[.package.name]
  ) then
    fail("release-order.json is not topological for the internal dependency DAG")
  else
    {
      version: $version,
      publishable_packages: ($order | length),
      release_order: $order
    }
  end
