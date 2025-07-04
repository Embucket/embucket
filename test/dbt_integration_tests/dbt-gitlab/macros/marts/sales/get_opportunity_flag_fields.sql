{% macro get_opportunity_flag_fields() %}
    {% set opportunity_flag_fields = [
        'is_won',
        'valid_deal_count',
        'is_closed',
        'is_edu_oss',
        'is_ps_opp',
        'is_public_sector_opp',
        'is_sao',
        'is_win_rate_calc',
        'is_net_arr_pipeline_created',
        'is_net_arr_closed_deal',
        'is_new_logo_first_order',
        'is_closed_won',
        'is_web_portal_purchase',
        'is_registration_from_portal',
        'is_stage_1_plus',
        'is_stage_3_plus',
        'is_stage_4_plus',
        'is_lost',
        'is_open',
        'is_active',
        'is_risky',
        'is_credit',
        'is_renewal',
        'is_refund',
        'is_deleted',
        'is_duplicate',
        'is_excluded_from_pipeline_created',
        'is_contract_reset',
        'fpa_master_bookings_flag',
        'is_comp_new_logo_override',
        'is_eligible_open_pipeline',
        'is_eligible_asp_analysis',
        'is_eligible_age_analysis',
        'is_eligible_churn_contraction',
        'is_booked_net_arr',
        'is_downgrade',
        'critical_deal_flag',
        'is_abm_tier_sao',
        'is_abm_tier_closed_won',
        'competitors_other_flag',
        'competitors_gitlab_core_flag',
        'competitors_none_flag',
        'competitors_github_enterprise_flag',
        'competitors_bitbucket_server_flag',
        'competitors_unknown_flag',
        'competitors_github_flag',
        'competitors_gitlab_flag',
        'competitors_jenkins_flag',
        'competitors_azure_devops_flag',
        'competitors_svn_flag',
        'competitors_bitbucket_flag',
        'competitors_atlassian_flag',
        'competitors_perforce_flag',
        'competitors_visual_studio_flag',
        'competitors_azure_flag',
        'competitors_amazon_code_commit_flag',
        'competitors_circleci_flag',
        'competitors_bamboo_flag',
        'competitors_aws_flag'
    ] %}
    
    {{ return(opportunity_flag_fields) }}
{% endmacro %}