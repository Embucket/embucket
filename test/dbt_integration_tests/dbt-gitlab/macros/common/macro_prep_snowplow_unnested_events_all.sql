{% macro macro_prep_snowplow_unnested_events_all(unioned_view) %}

SELECT
  event_id                                                                                                          AS event_id,
  derived_tstamp::TIMESTAMP                                                                                         AS behavior_at,
  {{ dbt_utils.generate_surrogate_key([
    'event',
    'event_name',
    'platform',
    'gsc_environment',
    'se_category',
    'se_action',
    'se_label',
    'se_property'
    ]) }}                                                                                                           AS dim_behavior_event_sk,
  event                                                                                                             AS event,
  event_name                                                                                                        AS event_name,
  se_action                                                                                                         AS event_action,
  se_category                                                                                                       AS event_category,
  se_label                                                                                                          AS event_label,
  IFF(REGEXP_LIKE(se_label, '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'),
                                          'identifier_containing_numbers', se_label)                                AS clean_event_label,
  se_property                                                                                                       AS event_property,
  se_value                                                                                                          AS event_value,
  is_staging_event                                                                                                  AS is_staging_event,
  platform                                                                                                          AS platform,
  gsc_pseudonymized_user_id                                                                                         AS gsc_pseudonymized_user_id,
  page_urlhost                                                                                                      AS page_url_host,
  app_id                                                                                                            AS app_id,
  domain_sessionid                                                                                                  AS session_id,
  lc_targeturl                                                                                                      AS link_click_target_url,
  NULLIF(lc_elementid,'')                                                                                           AS link_click_element_id,
  sf_formid                                                                                                         AS submit_form_id,
  cf_formid                                                                                                         AS change_form_id,
  cf_type                                                                                                           AS change_form_type,
  cf_elementid                                                                                                      AS change_form_element_id,
  ff_elementid                                                                                                      AS focus_form_element_id,
  ff_nodename                                                                                                       AS focus_form_node_name,
  br_family                                                                                                         AS browser_name,
  br_name                                                                                                           AS browser_major_version,
  br_version                                                                                                        AS browser_minor_version,
  br_lang                                                                                                           AS browser_language,
  br_renderengine                                                                                                   AS browser_engine,
  {{ dbt_utils.generate_surrogate_key([
    'br_family',
    'br_name',
    'br_version',
    'br_lang'])
    }}                                                                                                              AS dim_behavior_browser_sk,
  gsc_environment                                                                                                   AS environment,
  v_tracker                                                                                                         AS tracker_version,
  TRY_PARSE_JSON(contexts)::VARIANT                                                                                 AS contexts,
  dvce_created_tstamp::TIMESTAMP                                                                                    AS dvce_created_tstamp,
  collector_tstamp::TIMESTAMP                                                                                       AS collector_tstamp,
  domain_userid                                                                                                     AS user_snowplow_domain_id,
  domain_sessionidx::INT                                                                                            AS session_index,
  page_url                                                                                                          AS page_url,
  REGEXP_REPLACE(page_url, '^https?:\/\/')                                                                          AS page_url_host_path,
  page_urlscheme                                                                                                    AS page_url_scheme,
  page_urlpath                                                                                                      AS page_url_path,
  Null AS clean_url_path,
  page_urlfragment                                                                                                  AS page_url_fragment,
  page_urlquery                                                                                                     AS page_url_query,
  {{ dbt_utils.generate_surrogate_key([
    'page_url',
    'app_id',
    'page_url_scheme'
    ]) }}                                                                                                           AS dim_behavior_website_page_sk,
  gitlab_standard_context                                                                                           AS gitlab_standard_context,
  gsc_environment                                                                                                   AS gsc_environment,
  gsc_extra                                                                                                         AS gsc_extra,
  gsc_namespace_id                                                                                                  AS gsc_namespace_id,
  gsc_plan                                                                                                          AS gsc_plan,
  gsc_google_analytics_client_id                                                                                    AS gsc_google_analytics_client_id,
  gsc_project_id                                                                                                    AS gsc_project_id,
  gsc_source                                                                                                        AS gsc_source,
  gsc_is_gitlab_team_member                                                                                         AS gsc_is_gitlab_team_member,
  gsc_feature_enabled_by_namespace_ids                                                                              AS gsc_feature_enabled_by_namespace_ids,
  os_name                                                                                                           AS os_name,
  os_timezone                                                                                                       AS os_timezone,
  os_family                                                                                                         AS os,
  os_manufacturer                                                                                                   AS os_manufacturer,
  {{ dbt_utils.generate_surrogate_key([
    'os_name',
    'os_timezone'
    ]) }}                                                                                                           AS dim_behavior_operating_system_sk,
  dvce_type                                                                                                         AS device_type,
  -- Fix incorrectly identified mobile devices found in https://gitlab.com/gitlab-data/analytics/-/issues/22227
  IFF(dvce_type = 'Tablet' AND dvce_ismobile::BOOLEAN = TRUE, FALSE, dvce_ismobile::BOOLEAN)                        AS is_device_mobile,
  refr_medium                                                                                                       AS referrer_medium,
  refr_urlhost                                                                                                      AS referrer_url_host,
  refr_urlpath                                                                                                      AS referrer_url_path,
  refr_urlscheme                                                                                                    AS referrer_url_scheme,
  refr_urlquery                                                                                                     AS referrer_url_query,
  REGEXP_REPLACE(page_referrer, '^https?:\/\/')                                                                     AS referrer_url_host_path,
  page_referrer                                                                                                     AS referrer_url,
  {{ dbt_utils.generate_surrogate_key([
    'page_referrer',
    'app_id',
    'referrer_url_scheme'
    ]) }}                                                                                                           AS dim_behavior_referrer_page_sk,
  IFNULL(geo_city, 'Unknown')::VARCHAR                                                                              AS user_city,
  IFNULL(geo_country, 'Unknown')::VARCHAR                                                                           AS user_country,
  IFNULL(geo_region, 'Unknown')::VARCHAR                                                                            AS user_region,
  IFNULL(geo_region_name, 'Unknown')::VARCHAR                                                                       AS user_region_name,
  IFNULL(geo_timezone, 'Unknown')::VARCHAR                                                                          AS user_timezone_name,
  {{ dbt_utils.generate_surrogate_key([
    'user_city',
    'user_country',
    'user_region',
    'user_timezone_name'
    ]) }}                                                                                                           AS dim_user_location_sk,
  has_performance_timing_context,
  has_web_page_context,
  COALESCE(CONTAINS(contexts, 'iglu:com.gitlab/ci_build_failed/'), FALSE)::BOOLEAN                                  AS has_ci_build_failed_context,
  COALESCE(CONTAINS(contexts, 'iglu:com.gitlab/wiki_page_context/'), FALSE)::BOOLEAN                                AS has_wiki_page_context,
  has_gitlab_standard_context,
  COALESCE(CONTAINS(contexts, 'iglu:com.gitlab/email_campaigns/'), FALSE)::BOOLEAN                                  AS has_email_campaigns_context,
  has_gitlab_service_ping_context,
  COALESCE(CONTAINS(contexts, 'iglu:com.gitlab/design_management_context/'), FALSE)::BOOLEAN                        AS has_design_management_context,
  COALESCE(CONTAINS(contexts, 'iglu:com.gitlab/customer_standard/'), FALSE)::BOOLEAN                                AS has_customer_standard_context,
  COALESCE(CONTAINS(contexts, 'iglu:com.gitlab/secure_scan/'), FALSE)::BOOLEAN                                      AS has_secure_scan_context,
  has_gitlab_experiment_context,
  COALESCE(CONTAINS(contexts, 'iglu:com.gitlab/subscription_auto_renew/'), FALSE)::BOOLEAN                          AS has_subscription_auto_renew_context,
  COALESCE(CONTAINS(contexts, 'iglu:com.gitlab/code_suggestions_context/'), FALSE)::BOOLEAN                         AS has_code_suggestions_context,
  has_ide_extension_version_context,
  {{ dbt_utils.generate_surrogate_key([
    'has_performance_timing_context',
    'has_web_page_context',
    'has_ci_build_failed_context',
    'has_wiki_page_context',
    'has_gitlab_standard_context',
    'has_email_campaigns_context',
    'has_gitlab_service_ping_context',
    'has_design_management_context',
    'has_customer_standard_context',
    'has_secure_scan_context',
    'has_gitlab_experiment_context',
    'has_subscription_auto_renew_context',
    'has_code_suggestions_context',
    'has_ide_extension_version_context'
    ]) }}                                                                                                           AS dim_behavior_contexts_sk,
  ide_extension_version_context                                                                                     AS ide_extension_version_context,
  extension_name                                                                                                    AS extension_name,
  extension_version                                                                                                 AS extension_version,
  ide_name                                                                                                          AS ide_name,
  ide_vendor                                                                                                        AS ide_vendor,
  ide_version                                                                                                       AS ide_version,
  language_server_version                                                                                           AS language_server_version,
  gitlab_experiment_context                                                                                         AS gitlab_experiment_context,
  experiment_name                                                                                                   AS experiment_name,
  experiment_context_key                                                                                            AS experiment_context_key,
  experiment_variant                                                                                                AS experiment_variant,
  experiment_migration_keys                                                                                         AS experiment_migration_keys,
  code_suggestions_context                                                                                          AS code_suggestions_context,
  model_engine                                                                                                      AS model_engine,
  model_name                                                                                                        AS model_name,
  prefix_length                                                                                                     AS prefix_length,
  suffix_length                                                                                                     AS suffix_length,
  language                                                                                                          AS language,
  user_agent                                                                                                        AS user_agent,
  delivery_type                                                                                                     AS delivery_type,
  api_status_code                                                                                                   AS api_status_code,
  duo_namespace_ids                                                                                                 AS duo_namespace_ids,
  saas_namespace_ids                                                                                                AS saas_namespace_ids,
  namespace_ids                                                                                                     AS namespace_ids,
  instance_id                                                                                                       AS instance_id,
  host_name                                                                                                         AS host_name,
  is_streaming                                                                                                      AS is_streaming,
  gitlab_global_user_id                                                                                             AS gitlab_global_user_id,
  suggestion_source                                                                                                 AS suggestion_source,
  is_invoked                                                                                                        AS is_invoked,
  options_count                                                                                                     AS options_count,
  accepted_option                                                                                                   AS accepted_option,
  has_advanced_context                                                                                              AS has_advanced_context,
  is_direct_connection                                                                                              AS is_direct_connection,
  gitlab_service_ping_context                                                                                       AS gitlab_service_ping_context,
  redis_event_name                                                                                                  AS redis_event_name,
  key_path                                                                                                          AS key_path,
  data_source                                                                                                       AS data_source,
  performance_timing_context                                                                                        AS performance_timing_context,
  connect_end                                                                                                       AS connect_end,
  connect_start                                                                                                     AS connect_start,
  dom_complete                                                                                                      AS dom_complete,
  dom_content_loaded_event_end                                                                                      AS dom_content_loaded_event_end,
  dom_content_loaded_event_start                                                                                    AS dom_content_loaded_event_start,
  dom_interactive                                                                                                   AS dom_interactive,
  dom_loading                                                                                                       AS dom_loading,
  domain_lookup_end                                                                                                 AS domain_lookup_end,
  domain_lookup_start                                                                                               AS domain_lookup_start,
  fetch_start                                                                                                       AS fetch_start,
  load_event_end                                                                                                    AS load_event_end,
  load_event_start                                                                                                  AS load_event_start,
  navigation_start                                                                                                  AS navigation_start,
  redirect_end                                                                                                      AS redirect_end,
  redirect_start                                                                                                    AS redirect_start,
  request_start                                                                                                     AS request_start,
  response_end                                                                                                      AS response_end,
  response_start                                                                                                    AS response_start,
  secure_connection_start                                                                                           AS secure_connection_start,
  unload_event_end                                                                                                  AS unload_event_end,
  unload_event_start                                                                                                AS unload_event_start,
  instance_version                                                                                                  AS gsc_instance_version,
  correlation_id                                                                                                    AS gsc_correlation_id,
  total_context_size_bytes                                                                                          AS total_context_size_bytes,
  content_above_cursor_size_bytes                                                                                   AS content_above_cursor_size_bytes,
  content_below_cursor_size_bytes                                                                                   AS content_below_cursor_size_bytes,
  context_items                                                                                                     AS context_items,
  context_items_count                                                                                               AS context_items_count,
  input_tokens                                                                                                      AS input_tokens,
  output_tokens                                                                                                     AS output_tokens,
  context_tokens_sent                                                                                               AS context_tokens_sent,
  context_tokens_used                                                                                               AS context_tokens_used,
  debounce_interval                                                                                                 AS debounce_interval,
  interface                                                                                                         AS interface,
  client_type                                                                                                       AS client_type,
  client_name                                                                                                       AS client_name,
  client_version                                                                                                    AS client_version,
  feature_category                                                                                                  AS feature_category,
  region                                                                                                            AS region,
  resolution_strategy                                                                                               AS resolution_strategy
FROM unioned_view

{% endmacro %}