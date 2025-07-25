WITH source as (

	SELECT *
  	  FROM {{ source('greenhouse', 'dup_greenhouse_events') }}

), renamed as (

	SELECT

            --key
            id::NUMBER      AS greenhouse_event_id,

            --info
            name::varchar   AS greenhouse_event_name

	FROM source

)

SELECT *
FROM renamed
